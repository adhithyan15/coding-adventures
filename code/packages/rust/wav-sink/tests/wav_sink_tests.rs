use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::{SineOscillator, UniformSampler};
use pcm_audio::{PcmAudio, PcmError, PcmFormat};
use wav_sink::{
    from_wav_bytes, read_wav, to_wav_bytes, validate_wav_path, write_wav, WavSinkError,
    MAX_WAV_BYTES,
};

/// Self-cleaning scratch directory under the OS temp dir.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "wav-sink-test-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tone() -> PcmAudio {
    let osc = SineOscillator::new(440.0).unwrap();
    let buffer = UniformSampler::new(8000.0)
        .unwrap()
        .sample(&osc, 0.05)
        .unwrap();
    PcmAudio::from_sample_buffer(&buffer).unwrap()
}

fn is_invalid_path<T: std::fmt::Debug>(r: Result<T, WavSinkError>) -> bool {
    matches!(r, Err(WavSinkError::InvalidPath(_)))
}

#[cfg(unix)]
fn try_symlink(target: &Path, link: &Path, _dir: bool) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn try_symlink(target: &Path, link: &Path, dir: bool) -> bool {
    if dir {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    } else {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}

#[cfg(not(any(unix, windows)))]
fn try_symlink(_t: &Path, _l: &Path, _d: bool) -> bool {
    false
}

#[test]
fn round_trip_with_base_dir() {
    let scratch = Scratch::new("roundtrip");
    let audio = tone();
    let written = write_wav("tone.wav", &audio, Some(scratch.path())).unwrap();
    assert_eq!(written, scratch.path().join("tone.wav"));
    let decoded = read_wav("tone.wav", Some(scratch.path())).unwrap();
    assert_eq!(decoded.samples(), audio.samples());
    assert_eq!(decoded.format().sample_rate_hz(), 8000.0);
    // Overwrite is allowed and fully replaces content.
    let short = PcmAudio::from_i16(vec![1, 2, 3], PcmFormat::mono_16bit(8000.0).unwrap(), 0.0)
        .unwrap();
    write_wav(&written, &short, Some(scratch.path())).unwrap();
    assert_eq!(
        read_wav(&written, Some(scratch.path())).unwrap().samples(),
        &[1, 2, 3]
    );
}

#[test]
fn round_trip_without_base_dir_and_uppercase_extension() {
    let scratch = Scratch::new("nobase");
    let target = scratch.path().join("TONE.WAVE");
    let audio = tone();
    // Without a base_dir, ancestors must be symlink-free; skip if the OS temp
    // dir itself lives behind a symlink.
    if validate_wav_path(&target, None).is_err() {
        return;
    }
    write_wav(&target, &audio, None).unwrap();
    assert_eq!(read_wav(&target, None).unwrap().samples(), audio.samples());
}

#[test]
fn to_wav_bytes_matches_codec() {
    let audio = tone();
    assert_eq!(to_wav_bytes(&audio).unwrap(), audio.to_wav_bytes().unwrap());
    let decoded = from_wav_bytes(&to_wav_bytes(&audio).unwrap()).unwrap();
    assert_eq!(decoded.samples(), audio.samples());
}

#[test]
fn rejects_empty_nul_dotdot_and_bad_extension() {
    let scratch = Scratch::new("hostile");
    let base = Some(scratch.path());
    assert!(is_invalid_path(validate_wav_path(Path::new(""), None)));
    assert!(is_invalid_path(validate_wav_path(Path::new("a\0b.wav"), None)));
    assert!(is_invalid_path(validate_wav_path(Path::new("a\0b.wav"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("../x.wav"), None)));
    assert!(is_invalid_path(validate_wav_path(Path::new("../x.wav"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("a/../b.wav"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("x.txt"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("x"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("x.wav.exe"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new(".wav"), base)));
    assert!(is_invalid_path(validate_wav_path(Path::new("x.wav"), Some(Path::new("bad\0dir")))));
    assert!(is_invalid_path(validate_wav_path(Path::new("x.wav"), Some(&scratch.path().join("..")))));
    assert!(validate_wav_path(Path::new("x.wav"), base).is_ok());
    assert!(validate_wav_path(Path::new("x.WAVE"), base).is_ok());
}

#[test]
fn write_and_read_reject_hostile_paths() {
    let scratch = Scratch::new("hostile-io");
    let audio = tone();
    assert!(is_invalid_path(write_wav("a\0.wav", &audio, Some(scratch.path()))));
    assert!(is_invalid_path(write_wav("../escape.wav", &audio, Some(scratch.path()))));
    assert!(is_invalid_path(read_wav("../escape.wav", Some(scratch.path()))));
    assert!(!scratch.path().parent().unwrap().join("escape.wav").exists());
}

#[test]
fn base_dir_containment_is_enforced() {
    let scratch = Scratch::new("contain");
    let outside = Scratch::new("outside");
    let target = outside.path().join("x.wav");
    assert!(is_invalid_path(validate_wav_path(&target, Some(scratch.path()))));
    assert!(is_invalid_path(write_wav(&target, &tone(), Some(scratch.path()))));
    assert!(!target.exists());
    // Sibling directory sharing a string prefix must not count as inside.
    let sibling = PathBuf::from(format!("{}-evil", scratch.path().display()));
    fs::create_dir_all(&sibling).unwrap();
    let result = validate_wav_path(&sibling.join("x.wav"), Some(scratch.path()));
    let _ = fs::remove_dir_all(&sibling);
    assert!(is_invalid_path(result));
    // Inside absolute path is fine.
    assert!(validate_wav_path(&scratch.path().join("ok.wav"), Some(scratch.path())).is_ok());
}

#[test]
fn base_dir_must_be_an_existing_directory() {
    let scratch = Scratch::new("basedir");
    let file = scratch.path().join("plain.wav");
    fs::write(&file, b"x").unwrap();
    assert!(is_invalid_path(validate_wav_path(Path::new("a.wav"), Some(&file))));
    let missing = scratch.path().join("missing");
    assert!(matches!(
        validate_wav_path(Path::new("a.wav"), Some(&missing)),
        Err(WavSinkError::Io { .. })
    ));
}

#[test]
fn directory_target_is_rejected() {
    let scratch = Scratch::new("dirtarget");
    fs::create_dir(scratch.path().join("dir.wav")).unwrap();
    assert!(is_invalid_path(validate_wav_path(
        Path::new("dir.wav"),
        Some(scratch.path())
    )));
    assert!(write_wav("dir.wav", &tone(), Some(scratch.path())).is_err());
    assert!(read_wav("dir.wav", Some(scratch.path())).is_err());
}

#[test]
fn symlink_target_is_rejected() {
    let scratch = Scratch::new("symtarget");
    let real = scratch.path().join("real.wav");
    write_wav("real.wav", &tone(), Some(scratch.path())).unwrap();
    let link = scratch.path().join("link.wav");
    if !try_symlink(&real, &link, false) {
        eprintln!("skipping: cannot create symlinks here");
        return;
    }
    assert!(is_invalid_path(validate_wav_path(&link, Some(scratch.path()))));
    assert!(is_invalid_path(read_wav("link.wav", Some(scratch.path()))));
    let before = fs::read(&real).unwrap();
    assert!(is_invalid_path(write_wav(
        "link.wav",
        &PcmAudio::from_i16(vec![], PcmFormat::default(), 0.0).unwrap(),
        Some(scratch.path())
    )));
    assert_eq!(fs::read(&real).unwrap(), before, "symlink target untouched");
}

#[test]
fn symlinked_ancestor_is_rejected() {
    let scratch = Scratch::new("symancestor");
    let outside = Scratch::new("symoutside");
    let link_dir = scratch.path().join("linkdir");
    if !try_symlink(outside.path(), &link_dir, true) {
        eprintln!("skipping: cannot create symlinks here");
        return;
    }
    assert!(is_invalid_path(write_wav(
        "linkdir/x.wav",
        &tone(),
        Some(scratch.path())
    )));
    assert!(!outside.path().join("x.wav").exists());
    // Without a base_dir the same ancestor check applies.
    assert!(is_invalid_path(validate_wav_path(&link_dir.join("x.wav"), None)));
}

#[test]
fn corrupt_wav_files_are_errors_not_panics() {
    let scratch = Scratch::new("corrupt");
    let good = to_wav_bytes(&tone()).unwrap();
    let cases: Vec<Vec<u8>> = vec![
        vec![],
        b"RIFF".to_vec(),
        b"not a wav file at all, just text".to_vec(),
        good[..20].to_vec(),
        good[..good.len() - 1].to_vec(),
        {
            let mut b = good.clone();
            b[0..4].copy_from_slice(b"XXXX");
            b
        },
        {
            let mut b = good.clone();
            b[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
            b
        },
        {
            let mut b = good.clone();
            b[22..24].copy_from_slice(&0u16.to_le_bytes());
            b
        },
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let name = format!("c{i}.wav");
        fs::write(scratch.path().join(&name), bytes).unwrap();
        let result = read_wav(&name, Some(scratch.path()));
        assert!(
            matches!(result, Err(WavSinkError::Pcm(_))),
            "case {i}: {result:?}"
        );
        assert!(from_wav_bytes(bytes).is_err() || bytes.len() >= 44);
    }
}

#[test]
fn oversized_file_is_rejected_before_parsing() {
    let scratch = Scratch::new("oversize");
    let path = scratch.path().join("big.wav");
    let file = fs::File::create(&path).unwrap();
    file.set_len(MAX_WAV_BYTES as u64 + 1).unwrap();
    drop(file);
    let result = read_wav("big.wav", Some(scratch.path()));
    assert!(
        matches!(result, Err(WavSinkError::OversizedWav { max, .. }) if max == MAX_WAV_BYTES),
        "{result:?}"
    );
}

#[test]
fn oversized_in_memory_bytes_are_rejected() {
    let bytes = vec![0u8; MAX_WAV_BYTES + 1];
    assert!(matches!(
        from_wav_bytes(&bytes),
        Err(WavSinkError::OversizedWav { .. })
    ));
}

#[test]
fn error_display_and_conversion() {
    let e: WavSinkError = PcmError::MalformedWav("x".into()).into();
    assert!(e.to_string().contains("PCM codec error"));
    assert!(WavSinkError::InvalidPath("r".into()).to_string().contains("r"));
    assert!(WavSinkError::OversizedWav { size: 2, max: 1 }.to_string().contains("exceeds"));
    let _: &dyn std::error::Error = &e;
}

#[test]
fn pbt_round_trip_random_audio() {
    let scratch = Scratch::new("pbt-rt");
    let mut rng = SplitMix64::new(0x3A55_0001);
    for i in 0..40 {
        let count = rng.bounded(300) as usize;
        let samples: Vec<i16> = (0..count).map(|_| rng.next_u64() as u16 as i16).collect();
        let rate = [8000.0, 22_050.0, 44_100.0, 48_000.0][rng.bounded(4) as usize];
        let audio =
            PcmAudio::from_i16(samples.clone(), PcmFormat::mono_16bit(rate).unwrap(), 0.0).unwrap();
        let name = format!("rt{i}.wav");
        write_wav(&name, &audio, Some(scratch.path())).unwrap();
        let back = read_wav(&name, Some(scratch.path())).unwrap();
        assert_eq!(back.samples(), samples.as_slice());
        assert_eq!(back.format().sample_rate_hz(), rate);
    }
}

#[test]
fn pbt_random_bytes_never_panic() {
    let scratch = Scratch::new("pbt-bytes");
    let mut rng = SplitMix64::new(0xBAD0_0001);
    let good = to_wav_bytes(&tone()).unwrap();
    for i in 0..60 {
        let mut bytes = if rng.bounded(2) == 0 {
            (0..rng.bounded(200)).map(|_| rng.next_u64() as u8).collect::<Vec<u8>>()
        } else {
            good.clone()
        };
        // Random mutations.
        for _ in 0..rng.bounded(6) {
            if bytes.is_empty() {
                break;
            }
            let at = rng.bounded(bytes.len() as u64) as usize;
            bytes[at] = rng.next_u64() as u8;
        }
        if rng.bounded(4) == 0 {
            let keep = rng.bounded(bytes.len() as u64 + 1) as usize;
            bytes.truncate(keep);
        }
        let name = format!("f{i}.wav");
        fs::write(scratch.path().join(&name), &bytes).unwrap();
        let _ = read_wav(&name, Some(scratch.path()));
        let _ = from_wav_bytes(&bytes);
    }
}

#[test]
fn pbt_random_paths_never_panic_and_never_escape() {
    let scratch = Scratch::new("pbt-paths");
    let mut rng = SplitMix64::new(0xFA70_0001u64 & 0xFFFF_FFFF);
    let alphabet: Vec<&str> = vec![
        "a", "b", "..", ".", "/", "\\", "\0", "x.wav", ".wav", "wave", "C:", ":", "é", " ",
    ];
    for _ in 0..500 {
        let mut s = String::new();
        for _ in 0..rng.bounded(8) {
            s.push_str(alphabet[rng.bounded(alphabet.len() as u64) as usize]);
        }
        if let Ok(p) = validate_wav_path(Path::new(&s), Some(scratch.path())) {
            assert!(p.starts_with(scratch.path()), "{s:?} -> {p:?}");
            assert!(!s.contains('\0'));
            assert!(!p.components().any(|c| matches!(c, std::path::Component::ParentDir)));
        }
        let _ = validate_wav_path(Path::new(&s), None);
    }
}
