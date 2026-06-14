use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Config {
    pub redis_url: String,
    pub database_url: String,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub worker_count: usize,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub llm_api_key: String,
    pub queue_voter_inputs: String,
    pub queue_approved_actions: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let redis_host = env("REDIS_HOST", "redis");
        let redis_port = env("REDIS_PORT", "6379");
        let redis_password = env("REDIS_PASSWORD", "");
        let redis_url = if redis_password.is_empty() {
            format!("redis://{redis_host}:{redis_port}/")
        } else {
            format!("redis://:{redis_password}@{redis_host}:{redis_port}/")
        };

        Ok(Self {
            redis_url,
            database_url: env(
                "DATABASE_URL",
                "postgres://voter_app:changeme@postgres:5432/voter_intelligence",
            ),
            neo4j_uri: format!(
                "{}:{}",
                env("NEO4J_HOST", "neo4j"),
                env("NEO4J_BOLT_PORT", "7687"),
            ),
            neo4j_user: env("NEO4J_USER", "neo4j"),
            neo4j_password: env("NEO4J_PASSWORD", "changeme"),
            worker_count: env("WORKER_COUNT", "4").parse()?,
            llm_endpoint: env("LLM_ENDPOINT", "http://gpt-oss:8080/v1"),
            llm_model: env("LLM_MODEL", "openai/gpt-oss-120b"),
            llm_api_key: env("LLM_API_KEY", ""),
            queue_voter_inputs: env("QUEUE_VOTER_INPUTS", "queue:voter_inputs"),
            queue_approved_actions: env("QUEUE_APPROVED_ACTIONS", "queue:approved_actions"),
        })
    }
}

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.redis_url, "redis://redis:6379/");
        assert_eq!(cfg.worker_count, 4);
        assert_eq!(cfg.queue_voter_inputs, "queue:voter_inputs");
        assert_eq!(cfg.queue_approved_actions, "queue:approved_actions");
    }

    #[test]
    fn test_redis_url_with_password() {
        temp_env::with_var("REDIS_PASSWORD", Some("secret123"), || {
            let cfg = Config::from_env().unwrap();
            assert_eq!(cfg.redis_url, "redis://:secret123@redis:6379/");
        });
    }

    #[test]
    fn test_redis_url_no_password() {
        temp_env::with_var("REDIS_PASSWORD", None::<&str>, || {
            let cfg = Config::from_env().unwrap();
            assert_eq!(cfg.redis_url, "redis://redis:6379/");
        });
    }

    #[test]
    fn test_redis_custom_host_port() {
        temp_env::with_vars(
            vec![
                ("REDIS_HOST", Some("myredis")),
                ("REDIS_PORT", Some("6380")),
                ("REDIS_PASSWORD", Some("pass")),
            ],
            || {
                let cfg = Config::from_env().unwrap();
                assert_eq!(cfg.redis_url, "redis://:pass@myredis:6380/");
            },
        );
    }

    #[test]
    fn test_worker_count_custom() {
        temp_env::with_var("WORKER_COUNT", Some("8"), || {
            let cfg = Config::from_env().unwrap();
            assert_eq!(cfg.worker_count, 8);
        });
    }

    #[test]
    fn test_worker_count_invalid_falls_back() {
        temp_env::with_var("WORKER_COUNT", Some("not_a_number"), || {
            let result = Config::from_env();
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_custom_llm_endpoint() {
        temp_env::with_var("LLM_ENDPOINT", Some("https://custom.openrouter.ai/v1"), || {
            let cfg = Config::from_env().unwrap();
            assert_eq!(cfg.llm_endpoint, "https://custom.openrouter.ai/v1");
        });
    }

    #[test]
    fn test_default_llm_model() {
        assert_eq!(Config::from_env().unwrap().llm_model, "openai/gpt-oss-120b");
    }
}
