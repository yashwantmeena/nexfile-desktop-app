use std::fmt::Display;
use std::time::{Duration, Instant};

pub(crate) struct OperationLogger {
    scope: &'static str,
    subject: String,
    started: Instant,
}

impl OperationLogger {
    pub(crate) fn start(scope: &'static str, subject: impl Display) -> Self {
        let logger = Self {
            scope,
            subject: subject.to_string(),
            started: Instant::now(),
        };
        logger.write("START", "processing started");
        logger
    }

    pub(crate) fn stage(&self, name: &str) -> Instant {
        self.write("STEP", name);
        Instant::now()
    }

    pub(crate) fn stage_complete(&self, name: &str, stage_started: Instant, details: impl Display) {
        let details = details.to_string();
        let message = if details.is_empty() {
            format!(
                "{name} completed in {}",
                format_duration(stage_started.elapsed())
            )
        } else {
            format!(
                "{name} completed in {} | {details}",
                format_duration(stage_started.elapsed())
            )
        };
        self.write("DONE", &message);
    }

    pub(crate) fn cached(&self, output: impl Display) {
        self.write("SKIP", &format!("current output already exists | {output}"));
    }

    pub(crate) fn complete(&self, output: impl Display) {
        self.write(
            "OK",
            &format!(
                "processing completed in {} | output={output}",
                format_duration(self.started.elapsed())
            ),
        );
    }

    pub(crate) fn failed(&self, error: impl Display) {
        self.write(
            "FAIL",
            &format!(
                "processing failed after {} | {error}",
                format_duration(self.started.elapsed())
            ),
        );
    }

    fn write(&self, status: &str, message: &str) {
        eprintln!(
            "[{}][{}][+{}] {} | {}",
            self.scope,
            status,
            format_duration(self.started.elapsed()),
            self.subject,
            message
        );
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() == 0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}
