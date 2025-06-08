use std::collections::HashMap;
use statsd::Client;

pub struct MetricsService {
    client: Client,
}

impl MetricsService {
    pub fn new(host: &str, port: u16, prefix: &str) -> Self {
        let client = Client::new(format!("{}:{}", host, port), prefix).unwrap();
        Self { client }
    }

    pub fn increment(&self, metric: &str, tags: Option<HashMap<String, String>>) {
        let mut metric_name = metric.to_string();
        if let Some(tags) = tags {
            let tag_string = tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<String>>()
                .join(",");
            metric_name = format!("{}#{}", metric_name, tag_string);
        }
        self.client.incr(&metric_name);
    }

    pub fn gauge(&self, metric: &str, value: f64, tags: Option<HashMap<String, String>>) {
        let mut metric_name = metric.to_string();
        if let Some(tags) = tags {
            let tag_string = tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<String>>()
                .join(",");
            metric_name = format!("{}#{}", metric_name, tag_string);
        }
        self.client.gauge(&metric_name, value);
    }

    pub fn timing(&self, metric: &str, duration: std::time::Duration, tags: Option<HashMap<String, String>>) {
        let mut metric_name = metric.to_string();
        if let Some(tags) = tags {
            let tag_string = tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<String>>()
                .join(",");
            metric_name = format!("{}#{}", metric_name, tag_string);
        }
        self.client.timer(&metric_name, duration.as_millis() as f64);
    }
} 