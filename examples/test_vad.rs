use hound::{WavSpec, WavWriter};
use scribe::audio::{AudioCapture, VadConfig, VoiceActivityDetector};
use std::io::{self, Write};

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scribe VAD Interactive Test ===\n");

    // List available devices
    let devices = AudioCapture::list_devices();
    println!("Available audio input devices:");
    for (i, device) in devices.iter().enumerate() {
        println!("  {i}. {device}");
    }
    println!();

    // Create audio capture
    println!("Initializing audio capture at 16kHz mono...");
    let capture = AudioCapture::new(16000, None)?;
    println!("Sample rate: {} Hz", capture.sample_rate());
    println!();

    // Create VAD with default config
    let vad_config = VadConfig::default_16khz();
    println!("VAD Configuration:");
    println!("  Sample rate: {} Hz", vad_config.sample_rate);
    println!("  Aggressiveness: {}", vad_config.aggressiveness);
    println!("  Silence threshold: {} ms", vad_config.silence_ms);
    println!("  Min duration: {} ms", vad_config.min_duration_ms);
    println!("  Skip initial: {} ms", vad_config.skip_initial_ms);
    println!();

    let mut vad = VoiceActivityDetector::new(&vad_config)?;
    println!(
        "Frame size: {} samples ({}ms)\n",
        vad.frame_size(),
        vad.frame_duration_ms()
    );

    // Interactive loop
    println!("Press Enter to start recording...");
    println!("Then speak into your microphone.");
    println!(
        "Recording will stop after {} ms of silence.\n",
        vad_config.silence_ms
    );

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    println!("Recording started! Speak now...");
    println!(
        "(First {}ms skipped to avoid keyboard noise)",
        vad_config.skip_initial_ms
    );

    // Start audio stream
    let mut stream = capture.start_recording()?;

    // Collect frames
    let mut frames = Vec::new();
    let mut frame_count = 0;
    let mut speech_count = 0;
    let silence_threshold_frames = vad_config.silence_ms / vad.frame_duration_ms();
    let mut speech_detected = false;
    let mut silence_count = 0;
    let mut skip_count = vad_config.skip_initial_ms / vad.frame_duration_ms();

    #[allow(clippy::cast_precision_loss)]
    while let Some(frame) = stream.recv().await {
        frame_count += 1;

        // Skip initial frames
        if skip_count > 0 {
            skip_count -= 1;
            frames.push(frame);
            continue;
        }

        let is_speech = vad.is_voice_frame(&frame)?;

        if is_speech {
            speech_count += 1;
            silence_count = 0;
            speech_detected = true;
            print!("█");
        } else if speech_detected {
            silence_count += 1;
            print!("░");

            if silence_count >= silence_threshold_frames {
                println!("\n\nSilence threshold reached. Stopping...");
                frames.push(frame);
                break;
            }
        } else {
            print!("·");
        }

        io::stdout().flush()?;
        frames.push(frame);
    }

    stream.stop();

    println!("\nRecording stopped!");
    println!("Total frames: {frame_count}");
    println!("Speech frames: {speech_count}");
    println!(
        "Duration: {:.2}s",
        f64::from(frame_count) * f64::from(vad.frame_duration_ms()) / 1000.0
    );

    // Extract speech using VAD
    println!("\nProcessing with VAD...");
    let speech = vad.extract_speech_from_frames(frames)?;

    #[allow(clippy::cast_precision_loss)]
    if let Some(audio) = speech {
        let duration_ms = (audio.len() as f64 / f64::from(vad.sample_rate())) * 1000.0;
        println!("Speech segment extracted!");
        println!("  Samples: {}", audio.len());
        println!("  Duration: {duration_ms:.2}ms");

        // Save to WAV
        let output_path = "output.wav";
        println!("\nSaving to {output_path}...");

        let spec = WavSpec {
            channels: 1,
            sample_rate: vad.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec)?;
        for &sample in &audio {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;

        println!("Saved successfully!");
        println!("\nYou can play it with: aplay {output_path}");
    } else {
        println!("No speech detected or recording too short.");
        println!(
            "Try speaking for at least {}ms.",
            vad_config.min_duration_ms
        );
    }

    Ok(())
}
