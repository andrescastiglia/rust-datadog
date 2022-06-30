use crate::{config::Config, raw_span::RawSpan, span::Span};
use crossbeam_channel::Receiver;
use std::sync::Arc;

pub struct AgentClient {
    client_sender: crossbeam_channel::Sender<Vec<Span>>,
}

impl AgentClient {
    pub fn new(config: &Arc<Config>) -> Self {
        let num_cpus = num_cpus::get();
        let (client_sender, client_requests) = crossbeam_channel::bounded(num_cpus * 50);

        for _ in 0..num_cpus {
            let channel = client_requests.clone();
            let config = Arc::clone(config);

            std::thread::spawn(move || Self::thread_loop(&config, &channel));
        }

        Self { client_sender }
    }

    pub fn send(&self, stack: Vec<Span>) {
        self.client_sender.send(stack).unwrap_or_else(|_| {
            println!("Tracing send error: Channel closed!");
        });
    }

    fn thread_loop(config: &Arc<Config>, client_requests: &Receiver<Vec<Span>>) {
        // Loop as long as the channel is open
        while let Ok(stack) = client_requests.recv() {
            let count = stack.len();

            let spans: Vec<Vec<RawSpan>> = vec![stack
                .into_iter()
                .map(|span| RawSpan::from(&span, config))
                .collect()];

            match serde_json::to_string(&spans) {
                Err(e) => println!("Couldn't encode payload for datadog: {:?}", e),
                Ok(payload) => {
                    let req = attohttpc::post(config.endpoint())
                        .header("Content-Length", payload.len())
                        .header("Content-Type", "application/json")
                        .header("X-Datadog-Trace-Count", count)
                        .text(&payload);

                    match req.send() {
                        Ok(resp) if !resp.is_success() => {
                            println!("error from datadog agent: {:?}", resp);
                        }
                        Err(err) => println!("error sending traces to datadog: {:?}", err),
                        _ => {}
                    }
                }
            }
        }
    }
}
