//! Standard MIDI File (SMF Type 0/1) import and export.
//! Open format, no crates — so Harmonia can round-trip piano rolls with any DAW.

use crate::theory::NoteEvent;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MidiNote {
    pub start_sec: f64,
    pub duration_sec: f64,
    pub midi_note: u8,
    pub velocity: f64,
    pub channel: u8,
}

#[derive(Debug, Clone)]
pub struct MidiFile {
    pub ticks_per_quarter: u16,
    pub tempo_us_per_quarter: u32,
    pub notes: Vec<MidiNote>,
}

impl MidiFile {
    pub fn bpm(&self) -> f64 {
        60_000_000.0 / self.tempo_us_per_quarter.max(1) as f64
    }

    pub fn from_note_events(events: &[NoteEvent], bpm: f64, tuning_midi: impl Fn(&NoteEvent) -> u8) -> Self {
        let notes = events
            .iter()
            .map(|ev| MidiNote {
                start_sec: ev.start_sec,
                duration_sec: ev.duration_sec,
                midi_note: tuning_midi(ev),
                velocity: ev.velocity.clamp(0.0, 1.0),
                channel: (ev.voice_line as u8).min(15),
            })
            .collect();
        Self {
            ticks_per_quarter: 480,
            tempo_us_per_quarter: (60_000_000.0 / bpm.clamp(30.0, 260.0)) as u32,
            notes,
        }
    }

    pub fn to_note_events(&self, hz_of_midi: impl Fn(u8) -> f64) -> Vec<NoteEvent> {
        self.notes
            .iter()
            .map(|n| {
                let hz = hz_of_midi(n.midi_note);
                NoteEvent {
                    start_sec: n.start_sec,
                    duration_sec: n.duration_sec,
                    start_freq_hz: hz,
                    target_freq_hz: hz,
                    velocity: n.velocity,
                    voice_line: n.channel as usize,
                }
            })
            .collect()
    }
}

/// Approximate MIDI note number from frequency (12-TET A4=440).
pub fn hz_to_midi(hz: f64) -> u8 {
    let n = 69.0 + 12.0 * (hz.max(8.0) / 440.0).log2();
    n.round().clamp(0.0, 127.0) as u8
}

pub fn load_smf<P: AsRef<Path>>(path: P) -> Result<MidiFile, String> {
    let bytes = fs::read(path.as_ref()).map_err(|e| format!("MIDI read failed: {e}"))?;
    parse_smf(&bytes)
}

pub fn save_smf<P: AsRef<Path>>(path: P, midi: &MidiFile) -> Result<(), String> {
    let bytes = encode_smf(midi);
    fs::write(path.as_ref(), bytes).map_err(|e| format!("MIDI write failed: {e}"))
}

pub fn parse_smf(bytes: &[u8]) -> Result<MidiFile, String> {
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        return Err("Not a Standard MIDI File (missing MThd)".into());
    }
    let header_len = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if header_len < 6 || 8 + header_len > bytes.len() {
        return Err("Truncated MIDI header".into());
    }
    let division = u16::from_be_bytes(bytes[12..14].try_into().unwrap());
    let ticks_per_quarter = if division & 0x8000 == 0 {
        division.max(1)
    } else {
        480
    };

    let mut pos = 8 + header_len;
    let mut tempo_us = 500_000u32;
    let mut notes: Vec<MidiNote> = Vec::new();

    while pos + 8 <= bytes.len() {
        if &bytes[pos..pos + 4] != b"MTrk" {
            break;
        }
        let track_len = u32::from_be_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        pos += 8;
        let end = (pos + track_len).min(bytes.len());
        parse_track(&bytes[pos..end], ticks_per_quarter, &mut tempo_us, &mut notes)?;
        pos = end;
    }

    Ok(MidiFile {
        ticks_per_quarter,
        tempo_us_per_quarter: tempo_us,
        notes,
    })
}

fn parse_track(
    data: &[u8],
    tpq: u16,
    tempo_us: &mut u32,
    notes: &mut Vec<MidiNote>,
) -> Result<(), String> {
    let mut i = 0usize;
    let mut tick: u64 = 0;
    let mut running: u8 = 0;
    let mut sounding: Vec<(u8, u8, u64, u8)> = Vec::new(); // note, vel, start_tick, ch

    while i < data.len() {
        let (delta, n) = read_vlq(&data[i..])?;
        i += n;
        tick += delta as u64;
        if i >= data.len() {
            break;
        }
        let mut status = data[i];
        if status < 0x80 {
            status = running;
        } else {
            i += 1;
            running = status;
        }

        if status == 0xFF {
            if i >= data.len() {
                break;
            }
            let meta = data[i];
            i += 1;
            let (len, n) = read_vlq(&data[i..])?;
            i += n;
            if meta == 0x51 && len >= 3 && i + 3 <= data.len() {
                *tempo_us = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
            }
            i += len as usize;
            continue;
        }
        if status == 0xF0 || status == 0xF7 {
            let (len, n) = read_vlq(&data[i..])?;
            i += n + len as usize;
            continue;
        }

        let hi = status & 0xF0;
        let ch = status & 0x0F;
        match hi {
            0x90 | 0x80 => {
                if i + 1 >= data.len() {
                    break;
                }
                let note = data[i];
                let vel = data[i + 1];
                i += 2;
                if hi == 0x90 && vel > 0 {
                    sounding.push((note, vel, tick, ch));
                } else {
                    close_note(notes, &mut sounding, note, ch, tick, tpq, *tempo_us);
                }
            }
            0xA0 | 0xB0 | 0xE0 => i += 2,
            0xC0 | 0xD0 => i += 1,
            _ => {}
        }
    }
    for (note, vel, start, ch) in sounding {
        push_note(notes, note, vel, ch, start, tick.max(start + 1), tpq, *tempo_us);
    }
    Ok(())
}

fn close_note(
    notes: &mut Vec<MidiNote>,
    sounding: &mut Vec<(u8, u8, u64, u8)>,
    note: u8,
    ch: u8,
    tick: u64,
    tpq: u16,
    tempo: u32,
) {
    if let Some(idx) = sounding.iter().rposition(|(n, _, _, c)| *n == note && *c == ch) {
        let (n, vel, start, c) = sounding.remove(idx);
        push_note(notes, n, vel, c, start, tick.max(start + 1), tpq, tempo);
    }
}

fn push_note(
    notes: &mut Vec<MidiNote>,
    note: u8,
    vel: u8,
    ch: u8,
    start_tick: u64,
    end_tick: u64,
    tpq: u16,
    tempo: u32,
) {
    let us_per_tick = tempo as f64 / tpq.max(1) as f64;
    notes.push(MidiNote {
        start_sec: start_tick as f64 * us_per_tick / 1_000_000.0,
        duration_sec: (end_tick - start_tick) as f64 * us_per_tick / 1_000_000.0,
        midi_note: note,
        velocity: vel as f64 / 127.0,
        channel: ch,
    });
}

fn read_vlq(data: &[u8]) -> Result<(u32, usize), String> {
    let mut val = 0u32;
    for (i, &b) in data.iter().enumerate().take(4) {
        val = (val << 7) | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            return Ok((val, i + 1));
        }
    }
    Err("Invalid MIDI variable-length quantity".into())
}

fn write_vlq(mut val: u32) -> Vec<u8> {
    let mut bytes = vec![(val & 0x7F) as u8];
    val >>= 7;
    while val > 0 {
        bytes.push(((val & 0x7F) as u8) | 0x80);
        val >>= 7;
    }
    bytes.reverse();
    bytes
}

pub fn encode_smf(midi: &MidiFile) -> Vec<u8> {
    let tpq = midi.ticks_per_quarter.max(1);
    let us_per_tick = midi.tempo_us_per_quarter as f64 / tpq as f64;

    #[derive(Clone)]
    struct Ev {
        tick: u64,
        bytes: Vec<u8>,
    }
    let mut evs = Vec::new();
    evs.push(Ev {
        tick: 0,
        bytes: vec![
            0xFF,
            0x51,
            0x03,
            ((midi.tempo_us_per_quarter >> 16) & 0xFF) as u8,
            ((midi.tempo_us_per_quarter >> 8) & 0xFF) as u8,
            (midi.tempo_us_per_quarter & 0xFF) as u8,
        ],
    });
    for n in &midi.notes {
        let start = (n.start_sec * 1_000_000.0 / us_per_tick).round() as u64;
        let end = start + ((n.duration_sec * 1_000_000.0 / us_per_tick).round() as u64).max(1);
        let vel = (n.velocity * 127.0).round().clamp(1.0, 127.0) as u8;
        let ch = n.channel.min(15);
        evs.push(Ev {
            tick: start,
            bytes: vec![0x90 | ch, n.midi_note, vel],
        });
        evs.push(Ev {
            tick: end,
            bytes: vec![0x80 | ch, n.midi_note, 0x40],
        });
    }
    evs.sort_by_key(|e| e.tick);

    let mut track = Vec::new();
    let mut last = 0u64;
    for e in &evs {
        let delta = e.tick.saturating_sub(last) as u32;
        track.extend(write_vlq(delta));
        track.extend_from_slice(&e.bytes);
        last = e.tick;
    }
    track.extend(write_vlq(0));
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    let mut out = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes()); // Type 0
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&tpq.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend(track);
    out
}
