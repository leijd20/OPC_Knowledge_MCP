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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // 测试 AuditLogger::new()
    #[test]
    fn test_new_creates_log_directory() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("subdir").join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let _logger = AuditLogger::new(log_path_str);

        // 父目录应当被创建
        assert!(log_path.parent().unwrap().exists());
    }

    #[test]
    fn test_new_stores_log_path() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let logger = AuditLogger::new(log_path_str.clone());

        assert_eq!(logger.log_path, log_path_str);
    }

    // 测试 AuditLogger::log()
    #[test]
    fn test_log_writes_log_entry() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let logger = AuditLogger::new(log_path_str);
        logger.log("admin", "rag_query", "test query", "success");

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("user=admin"));
        assert!(content.contains("tool=rag_query"));
        assert!(content.contains("params=test query"));
        assert!(content.contains("result=success"));
    }

    #[test]
    fn test_log_timestamp_format() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let logger = AuditLogger::new(log_path_str);
        logger.log("user1", "tool1", "params1", "result1");

        let content = fs::read_to_string(&log_path).unwrap();
        // RFC3339 格式：2026-05-03T...
        assert!(content.starts_with("["));
        assert!(content.contains("T")); // RFC3339 时间分隔符
    }

    #[test]
    fn test_log_appends_to_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let logger = AuditLogger::new(log_path_str);
        logger.log("user1", "tool1", "params1", "result1");
        logger.log("user2", "tool2", "params2", "result2");

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("user=user1"));
        assert!(content.contains("user=user2"));
        // 应该有两行
        assert_eq!(content.lines().count(), 2);
    }

    #[test]
    fn test_log_io_error_does_not_panic() {
        // 使用一个无效路径（包含不能打开的字符）
        let logger = AuditLogger::new("/nonexistent/dir/that/cannot/exist/audit.log".to_string());

        // 应该静默失败，不 panic
        logger.log("user", "tool", "params", "result");
    }

    #[test]
    fn test_log_with_special_characters() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        let log_path_str = log_path.to_str().unwrap().to_string();

        let logger = AuditLogger::new(log_path_str);
        logger.log("user", "tool", "query with spaces and = signs", "ok");

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("query with spaces"));
    }
}
