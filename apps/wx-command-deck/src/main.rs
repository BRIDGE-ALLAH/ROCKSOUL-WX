use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut api_url = use_signal(|| "http://127.0.0.1:8080".to_owned());
    let mut api_key = use_signal(String::new);
    let mut objective = use_signal(String::new);
    let mut classification = use_signal(|| "public".to_owned());
    let mut execution_location = use_signal(|| "local".to_owned());
    let mut allow_egress = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut result = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "app-shell",
            aside { class: "sidebar",
                div { class: "brand",
                    div { class: "brand-mark", "R" }
                    div {
                        div { class: "brand-name", "ROCKSOUL" }
                        div { class: "brand-subtitle", "WORKLOAD EXCHANGE" }
                    }
                }
                div { class: "nav-label", "COMMAND" }
                a { class: "nav-item active", href: "#overview", "◈  Overview" }
                a { class: "nav-item", href: "#workloads", "▦  Workload lab" }
                a { class: "nav-item", href: "#execution", "⇄  Execution" }
                a { class: "nav-item", href: "#governance", "◇  Governance" }
                a { class: "nav-item", href: "#receipts", "⌁  Evidence ledger" }
                div { class: "sidebar-bottom",
                    div { class: "connection-dot" }
                    span { "LOCAL CONTROL PLANE" }
                }
            }
            main { class: "main-content",
                header { class: "topbar",
                    div {
                        div { class: "eyebrow", "OUTCOME-DRIVEN INTELLIGENCE FABRIC" }
                        h1 { "Command Deck" }
                    }
                    div { class: "topbar-right",
                        span { class: "status-pill", "●  LIVE API MODE" }
                        span { class: "version", "v0.2.1" }
                    }
                }
                section { class: "hero",
                    div { class: "hero-copy",
                        div { class: "eyebrow accent", "THE WORKLOAD IS THE UNIT OF COMPUTE" }
                        h2 { "Don't route prompts.\nRoute outcomes." }
                        p { "Compile an objective into a governed execution. Keep provider credentials on the server, enforce data boundaries, and inspect the receipt for every completed request." }
                        div { class: "hero-tags",
                            span { "RUST CORE" }
                            span { "POLICY FIRST" }
                            span { "OPENAI-COMPATIBLE PROVIDERS" }
                        }
                    }
                    div { class: "hero-orbit",
                        div { class: "orbit orbit-one" }
                        div { class: "orbit orbit-two" }
                        div { class: "orbit-core", "WX" }
                        div { class: "orbit-node node-a", "INTENT" }
                        div { class: "orbit-node node-b", "POLICY" }
                        div { class: "orbit-node node-c", "RECEIPT" }
                    }
                }
                section { class: "metrics-grid",
                    MetricCard { label: "EXECUTION MODE", value: "LIVE", note: "Workload API enabled" }
                    MetricCard { label: "DATA POLICY", value: "ENFORCED", note: "Server-side policy checks" }
                    MetricCard { label: "PROVIDER", value: "CONFIGURED", note: "Selected by gateway environment" }
                    MetricCard { label: "RECEIPTS", value: "LOCAL", note: "Persisted by workload ID" }
                }
                section { class: "content-grid",
                    div { class: "panel workload-panel", id: "workloads",
                        div { class: "panel-heading",
                            div {
                                div { class: "eyebrow", "WORKLOAD LAB" }
                                h3 { "Execute an objective" }
                            }
                            span { class: "panel-index", "01 / EXECUTE" }
                        }
                        p { class: "panel-description", "The browser submits a workload contract. The gateway checks policy, selects its configured provider, executes once, evaluates supported acceptance checks, and stores a receipt." }

                        label { "GATEWAY URL" }
                        input {
                            class: "text-input",
                            value: "{api_url}",
                            placeholder: "http://127.0.0.1:8080",
                            oninput: move |event| api_url.set(event.value())
                        }
                        label { "GATEWAY API KEY (OPTIONAL FOR LOOPBACK DEVELOPMENT)" }
                        input {
                            class: "text-input",
                            r#type: "password",
                            value: "{api_key}",
                            placeholder: "Set WX_API_KEY on the gateway for protected deployments",
                            oninput: move |event| api_key.set(event.value())
                        }
                        label { "DATA CLASSIFICATION" }
                        select {
                            class: "text-input",
                            value: "{classification}",
                            onchange: move |event| classification.set(event.value()),
                            option { value: "public", "Public" }
                            option { value: "internal", "Internal" }
                            option { value: "confidential", "Confidential" }
                            option { value: "restricted", "Restricted" }
                        }
                        label { "EXECUTION LOCATION" }
                        select {
                            class: "text-input",
                            value: "{execution_location}",
                            onchange: move |event| execution_location.set(event.value()),
                            option { value: "local", "Local (Ollama / LM Studio)" }
                            option { value: "private_cloud", "Private cloud" }
                            option { value: "public_cloud", "Public cloud" }
                        }
                        label { "WORKLOAD OBJECTIVE / INPUT" }
                        textarea {
                            value: "{objective}",
                            placeholder: "e.g. Summarize this public release note and list the three most important changes...",
                            oninput: move |event| {
                                objective.set(event.value());
                                result.set(None);
                                error.set(None);
                            }
                        }
                        label { class: "checkbox-label",
                            input {
                                r#type: "checkbox",
                                checked: allow_egress(),
                                onchange: move |event| allow_egress.set(event.checked())
                            }
                            " I explicitly allow raw data egress to the provider configured on the gateway."
                        }
                        div { class: "form-footer",
                            span { "No provider secret is stored in this browser." }
                            button {
                                class: "primary-button",
                                disabled: objective().trim().is_empty() || busy(),
                                onclick: move |_| {
                                    let base = api_url().trim().trim_end_matches('/').to_owned();
                                    let secret = api_key();
                                    let input = objective();
                                    let data_classification = classification();
                                    let selected_location = execution_location();
                                    let egress = allow_egress();
                                    let mut busy_signal = busy;
                                    let mut result_signal = result;
                                    let mut error_signal = error;

                                    spawn(async move {
                                        busy_signal.set(true);
                                        result_signal.set(None);
                                        error_signal.set(None);

                                        let allowed_execution = vec![selected_location.as_str()];
                                        let payload = serde_json::json!({
                                            "contract": {
                                                "objective": input,
                                                "data_classification": data_classification,
                                                "raw_data_egress": if egress { "allow" } else { "deny" },
                                                "allowed_execution": allowed_execution,
                                                "acceptance_criteria": ["non_empty_output"]
                                            },
                                            "input": input
                                        });

                                        let url = format!("{base}/v1/workloads/execute");
                                        let mut builder = gloo_net::http::Request::post(&url)
                                            .header("content-type", "application/json");
                                        if !secret.trim().is_empty() {
                                            let bearer = format!("Bearer {secret}");
                                            builder = builder.header("authorization", &bearer);
                                        }

                                        let response = match builder.body(payload.to_string()) {
                                            Ok(request) => request.send().await,
                                            Err(request_error) => {
                                                error_signal.set(Some(format!("Could not build request: {request_error}")));
                                                busy_signal.set(false);
                                                return;
                                            }
                                        };

                                        match response {
                                            Ok(response) => {
                                                let status = response.status();
                                                match response.text().await {
                                                    Ok(body) if (200..300).contains(&status) => {
                                                        let pretty = serde_json::from_str::<serde_json::Value>(&body)
                                                            .ok()
                                                            .and_then(|value| serde_json::to_string_pretty(&value).ok())
                                                            .unwrap_or(body);
                                                        result_signal.set(Some(pretty));
                                                    }
                                                    Ok(body) => {
                                                        error_signal.set(Some(format!("Gateway returned HTTP {status}: {body}")));
                                                    }
                                                    Err(read_error) => {
                                                        error_signal.set(Some(format!("Could not read gateway response: {read_error}")));
                                                    }
                                                }
                                            }
                                            Err(network_error) => {
                                                error_signal.set(Some(format!("Gateway connection failed: {network_error}")));
                                            }
                                        }
                                        busy_signal.set(false);
                                    });
                                },
                                if busy() { "Executing..." } else { "Compile & Execute  →" }
                            }
                        }
                        if let Some(message) = error() {
                            div { class: "error-output", "{message}" }
                        }
                        if let Some(body) = result() {
                            div { class: "result-block",
                                div { class: "eyebrow accent", "EXECUTION RESULT" }
                                pre { "{body}" }
                            }
                        }
                    }
                    div { class: "panel principles-panel", id: "governance",
                        div { class: "eyebrow", "RUNTIME PRINCIPLES" }
                        h3 { "Authority stays outside the model." }
                        Principle { number: "01", title: "Policy beats prompts", body: "The gateway—not the LLM—enforces data classification and egress boundaries." }
                        Principle { number: "02", title: "Configured provider only", body: "The browser cannot redirect inference to an arbitrary URL or read provider credentials." }
                        Principle { number: "03", title: "Evidence is scoped", body: "Receipts record execution metadata and checks; they do not claim universal proof of truth." }
                        div { class: "execution-note",
                            strong { "Provider setup" }
                            p { "Configure WX_UPSTREAM_BASE_URL, WX_UPSTREAM_MODEL, and WX_UPSTREAM_LOCATION on the gateway. For local development, Ollama's OpenAI-compatible endpoint is the default." }
                        }
                    }
                }
                footer { "ROCKSOUL-WX · WORKLOAD EXECUTION · RECEIPTS PERSISTED LOCALLY" }
            }
        }
    }
}

#[component]
fn MetricCard(label: &'static str, value: &'static str, note: &'static str) -> Element {
    rsx! {
        div { class: "metric-card",
            div { class: "metric-label", "{label}" }
            div { class: "metric-value", "{value}" }
            div { class: "metric-note", "{note}" }
        }
    }
}

#[component]
fn Principle(number: &'static str, title: &'static str, body: &'static str) -> Element {
    rsx! {
        div { class: "principle-row",
            div { class: "principle-number", "{number}" }
            div {
                div { class: "principle-title", "{title}" }
                div { class: "principle-body", "{body}" }
            }
        }
    }
}
