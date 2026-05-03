use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct AuditLogger {
    log_path: String,
}

impl AuditLogger {
    pub fn new(log_path: String) -> Self {
        // 确保日志目录存在
        if let Some(parent) = Path::new(&log_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Self { log_path }
    }

    pub fn log(&self, user: &str, tool: &str, params: &str, result: &str) {
        let timestamp = Utc::now().to_rfc3339();
        let log_entry = format!(
            "[{}] user={} tool={} params={} result={}\n",
            timestamp, user, tool, params, result
        );

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = file.write_all(log_entry.as_bytes());
        }
    }
}
