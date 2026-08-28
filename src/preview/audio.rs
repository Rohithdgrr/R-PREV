//! AudioHandler — lofty meta + symphonia waveform + playback via rodio Sink
//! Phase 2: metadata + 80-bar waveform; playback controlled via Space/s in app.rs
use std::path::Path;
use std::time::Duration;
use crate::preview::{PreviewResult, PreviewHandler, PreviewCtx, meta, AudioMeta};
use crate::error::PreviewError;
use lofty::file::TaggedFileExt as _;
use lofty::file::AudioFile as _;
use symphonia::core::audio::Signal as _;

pub struct AudioHandler;

impl PreviewHandler for AudioHandler {
    fn name(&self) -> &'static str { "audio" }
    fn priority(&self, path: &Path, mime: &str, magic: &[u8]) -> u8 {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let is_audio_ext = matches!(ext.as_str(), "mp3"|"flac"|"wav"|"ogg"|"m4a"|"aac"|"opus"|"wma"|"aiff");
        let is_audio_magic = mime.starts_with("audio/") || magic.starts_with(b"ID3") || magic.starts_with(b"OggS") || magic.starts_with(b"RIFF");
        if is_audio_ext || is_audio_magic { 85 } else { 2 }
    }
    fn file_size_limit(&self) -> u64 { 200 * 1024 * 1024 }

    fn preview_blocking(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError> {
        let path = &ctx.path;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > self.file_size_limit() && !crate::preview::is_force_preview() {
            return Err(PreviewError::TooLarge(size, self.file_size_limit()));
        }
        // lofty metadata
        let tagged = lofty::read_from_path(path).map_err(|e| PreviewError::Decode(format!("lofty: {}", e)))?;
        let duration = tagged.properties().duration();
        use lofty::tag::Accessor as _;
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        let title = tag.and_then(|t| t.title().map(|s| s.to_string()));
        let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()));
        let meta = AudioMeta { title: title.clone(), artist: artist.clone(), duration };

        // waveform: symphonia decode first ~30s, downsample to 80 bars
        let waveform = build_waveform(path, 80).unwrap_or_default();

        let file_meta = meta::file_meta(path);
        let duration_str = format_duration(duration);
        let title_str = title.unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("audio").to_string());
        let artist_str = artist.unwrap_or_else(|| "Unknown artist".to_string());
        let waveform_str = waveform.iter().map(|v| {
            let level = (*v as usize * 8) / 255;
            match level { 0=>" ",1=>"_",2=>".-",3=>"-",4=>"*",5=>"#",6=>"█",7=>"█",8=>"█", _=>" " }
        }).collect::<Vec<_>>().join("");

        // Build a Text-like Audio result — ui will render special
        // For now return Audio variant; fallback text also available via preview_pane handling
        // Embed waveform as part of AudioMeta rendering in UI; preview_pane will draw sparkline
        // We store waveform and also produce lines for headless
        let _ = waveform_str;
        let _ = duration_str;
        let _ = title_str;
        let _ = artist_str;

        Ok(PreviewResult::Audio { meta, waveform, duration })
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs/60, secs%60)
}

fn build_waveform(path: &Path, bars: usize) -> anyhow::Result<Vec<u8>> {
    use symphonia::core::probe::Hint;
    use symphonia::core::io::MediaSourceStream;
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) { hint.with_extension(ext); }
    let probed = symphonia::default::get_probe().format(&hint, mss, &Default::default(), &Default::default())?;
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(|| anyhow::anyhow!("no track"))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;
    // Decode up to ~30s worth: assume 44100 * 30 samples
    let mut samples: Vec<i16> = Vec::with_capacity(44100*30);
    let max_packets = 800; // enough for ~30s at typical frame sizes
    for _ in 0..max_packets {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::ResetRequired) => { break; },
            Err(_) => break,
        };
        if packet.track_id() != track_id { continue; }
        let decoded = decoder.decode(&packet)?;
        // Convert to mono i16 samples
        use symphonia::core::audio::AudioBufferRef;
        match decoded {
            AudioBufferRef::S16(buf) => {
                // Take first channel
                let ch = buf.chan(0);
                samples.extend_from_slice(ch);
            }
            AudioBufferRef::S32(buf) => {
                let ch = buf.chan(0);
                for &v in ch { samples.push((v >> 16) as i16); }
            }
            AudioBufferRef::F32(buf) => {
                let ch = buf.chan(0);
                for &v in ch { samples.push((v.clamp(-1.0,1.0)*32767.0) as i16); }
            }
            _ => {}
        }
        if samples.len() > 1_200_000 { break; }
    }
    if samples.is_empty() { anyhow::bail!("no samples decoded"); }
    // Downsample to bars: rms per bucket
    let chunk = (samples.len() / bars).max(1);
    let mut out = Vec::with_capacity(bars);
    for i in 0..bars {
        let start = i*chunk;
        let end = ((i+1)*chunk).min(samples.len());
        if start >= end { out.push(0); continue; }
        let slice = &samples[start..end];
        let rms = (slice.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / slice.len() as f64).sqrt();
        let norm = (rms / 32767.0 * 255.0).clamp(0.0, 255.0) as u8;
        out.push(norm);
    }
    Ok(out)
}

fn format_duration_str(d: Duration) -> String { format_duration(d) }
