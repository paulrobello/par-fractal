use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoFormat {
    MP4,
    WebM,
    GIF,
}

impl VideoFormat {
    pub fn extension(&self) -> &str {
        match self {
            VideoFormat::MP4 => "mp4",
            VideoFormat::WebM => "webm",
            VideoFormat::GIF => "gif",
        }
    }

    #[allow(dead_code)]
    pub fn codec(&self) -> &str {
        match self {
            VideoFormat::MP4 => "libx264",
            VideoFormat::WebM => "libvpx-vp9",
            VideoFormat::GIF => "gif",
        }
    }

    #[allow(dead_code)]
    pub fn pixel_format(&self) -> &str {
        match self {
            VideoFormat::MP4 => "yuv420p",
            VideoFormat::WebM => "yuv420p",
            VideoFormat::GIF => "rgb24", // GIF uses RGB
        }
    }

    #[allow(dead_code)]
    pub fn is_gif(&self) -> bool {
        matches!(self, VideoFormat::GIF)
    }
}

pub struct VideoRecorder {
    width: u32,
    height: u32,
    fps: u32,
    format: VideoFormat,
    frame_sender: Option<Sender<Vec<u8>>>,
    encoder_thread: Option<thread::JoinHandle<()>>,
    is_recording: bool,
    frame_count: u32,
    filename: String,
}

impl VideoRecorder {
    pub fn new(width: u32, height: u32, fps: u32, format: VideoFormat) -> Self {
        Self {
            width,
            height,
            fps,
            format,
            frame_sender: None,
            encoder_thread: None,
            is_recording: false,
            frame_count: 0,
            filename: String::new(),
        }
    }

    pub fn start_recording(&mut self, filename: String) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".to_string());
        }

        // Check if ffmpeg is available
        if !self.check_ffmpeg_available() {
            return Err("ffmpeg not found. Please install ffmpeg to record videos.".to_string());
        }

        println!(
            "Starting video recording: {}x{} @ {}fps, format: {:?}",
            self.width, self.height, self.fps, self.format
        );

        self.filename = filename.clone();
        self.frame_count = 0;

        // Create channel for frame data
        let (sender, receiver) = bounded::<Vec<u8>>(30); // Buffer up to 30 frames
        self.frame_sender = Some(sender);

        // Spawn encoder thread
        let width = self.width;
        let height = self.height;
        let fps = self.fps;
        let format = self.format;

        let encoder_thread = thread::spawn(move || {
            if let Err(e) =
                Self::encoder_thread_main(width, height, fps, format, receiver, &filename)
            {
                eprintln!("Video encoder error: {}", e);
            }
        });

        self.encoder_thread = Some(encoder_thread);
        self.is_recording = true;

        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<String, String> {
        if !self.is_recording {
            return Err("Not recording".to_string());
        }

        println!("Stopping video recording ({} frames)...", self.frame_count);

        // Drop sender to signal encoder thread to finish
        self.frame_sender = None;

        // Wait for encoder thread to finish
        if let Some(thread) = self.encoder_thread.take() {
            if let Err(e) = thread.join() {
                eprintln!("Encoder thread panicked: {:?}", e);
            }
        }

        self.is_recording = false;
        println!("Video saved to {}", self.filename);

        Ok(self.filename.clone())
    }

    pub fn add_frame(&mut self, frame_data: Vec<u8>) -> Result<(), String> {
        if !self.is_recording {
            return Err("Not recording".to_string());
        }

        if let Some(sender) = &self.frame_sender {
            // Try to send the frame, drop if channel is full (skip frame)
            if sender.try_send(frame_data).is_ok() {
                self.frame_count += 1;
            } else {
                println!("Warning: Frame buffer full, skipping frame");
            }
        }

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    fn check_ffmpeg_available(&self) -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    fn encoder_thread_main(
        width: u32,
        height: u32,
        fps: u32,
        format: VideoFormat,
        receiver: Receiver<Vec<u8>>,
        filename: &str,
    ) -> Result<(), String> {
        // Build ffmpeg command based on format
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-y", // Overwrite output file
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgba",
            "-video_size",
            &format!("{}x{}", width, height),
            "-framerate",
            &fps.to_string(),
            "-i",
            "pipe:0", // Read from stdin
        ]);

        // Add format-specific encoding options
        match format {
            VideoFormat::MP4 => {
                cmd.args([
                    "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "medium", "-crf",
                    "23", // Quality (lower = better, 23 is default)
                ]);
            }
            VideoFormat::WebM => {
                cmd.args([
                    "-c:v",
                    "libvpx-vp9",
                    "-pix_fmt",
                    "yuv420p",
                    "-b:v",
                    "2M", // Bitrate for VP9
                    "-quality",
                    "good",
                    "-speed",
                    "2",
                ]);
            }
            VideoFormat::GIF => {
                // GIF encoding with palette optimization
                // Use split filter to generate palette and apply it in one pass
                cmd.args([
                    "-filter_complex",
                    "[0:v] split [a][b];[a] palettegen=stats_mode=diff:max_colors=256 [p];[b][p] paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle",
                    "-loop",
                    "0", // Loop forever
                ]);
            }
        }

        cmd.arg(filename)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut ffmpeg = cmd
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg: {}", e))?;

        let mut stdin = ffmpeg.stdin.take().ok_or("Failed to open ffmpeg stdin")?;

        // Process frames from receiver
        let mut frame_count = 0;
        while let Ok(frame_data) = receiver.recv() {
            if let Err(e) = stdin.write_all(&frame_data) {
                eprintln!("Failed to write frame to ffmpeg: {}", e);
                break;
            }
            frame_count += 1;
        }

        // Close stdin to signal end of input
        drop(stdin);

        // Wait for ffmpeg to finish
        let output = ffmpeg
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("ffmpeg stderr: {}", stderr);
            return Err(format!("ffmpeg failed with status: {}", output.status));
        }

        let format_name = match format {
            VideoFormat::MP4 => "MP4 video",
            VideoFormat::WebM => "WebM video",
            VideoFormat::GIF => "GIF animation",
        };

        println!(
            "{} encoding complete: {} frames written to {}",
            format_name, frame_count, filename
        );

        Ok(())
    }
}

impl Drop for VideoRecorder {
    fn drop(&mut self) {
        if self.is_recording {
            let _ = self.stop_recording();
        }
    }
}
