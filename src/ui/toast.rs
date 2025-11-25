/// Toast notification for displaying temporary messages
#[derive(Clone)]
pub struct Toast {
    pub(super) message: String,
    pub(super) file_path: Option<String>,
    pub(super) created_at: web_time::Instant,
    pub(super) duration_secs: f32,
}

impl Toast {
    pub fn with_file_path(message: String, file_path: String, duration_secs: f32) -> Self {
        Self {
            message,
            file_path: Some(file_path),
            created_at: web_time::Instant::now(),
            duration_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs_f32() > self.duration_secs
    }

    pub fn opacity(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_secs_f32();
        let fade_duration = 0.5;

        if elapsed > self.duration_secs - fade_duration {
            let fade_progress = (self.duration_secs - elapsed) / fade_duration;
            fade_progress.max(0.0)
        } else {
            1.0
        }
    }
}
