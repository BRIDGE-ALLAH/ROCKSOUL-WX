use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut objective = use_signal(String::new);
    let mut prepared = use_signal(|| false);

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
                a { class: "nav-item", href: "#workloads", "▦  Workloads" }
                a { class: "nav-item", href: "#exchange", "⇄  Intelligence exchange" }
                a { class: "nav-item", href: "#sovereignty", "◇  Sovereignty" }
                a { class: "nav-item", href: "#evidence", "⌁  Evidence ledger" }
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
                        span { class: "status-pill", "●  PLANNING MODE" }
                        span { class: "version", "v0.1.0" }
                    }
                }
                section { class: "hero",
                    div { class: "hero-copy",
                        div { class: "eyebrow accent", "THE WORKLOAD IS THE UNIT OF COMPUTE" }
                        h2 { "Don't route prompts.\nRoute outcomes." }
                        p { "Compile objectives into governed plans. Choose where intelligence is allowed to run. Verify the work before trusting the result." }
                        div { class: "hero-tags",
                            span { "RUST CORE" }
                            span { "POLICY FIRST" }
                            span { "PLANNING FOUNDATION" }
                        }
                    }
                    div { class: "hero-orbit",
                        div { class: "orbit orbit-one" }
                        div { class: "orbit orbit-two" }
                        div { class: "orbit-core", "WX" }
                        div { class: "orbit-node node-a", "INTENT" }
                        div { class: "orbit-node node-b", "POLICY" }
                        div { class: "orbit-node node-c", "PROOF" }
                    }
                }
                section { class: "metrics-grid",
                    MetricCard { label: "ACTIVE WORKLOADS", value: "—", note: "Runtime not connected" }
                    MetricCard { label: "VERIFIED OUTCOMES", value: "—", note: "Verification pipeline pending" }
                    MetricCard { label: "POLICY VIOLATIONS", value: "—", note: "Live telemetry pending" }
                    MetricCard { label: "INFERENCE SPEND", value: "—", note: "No provider calls in this build" }
                }
                section { class: "content-grid",
                    div { class: "panel workload-panel",
                        div { class: "panel-heading",
                            div {
                                div { class: "eyebrow", "WORKLOAD LAB" }
                                h3 { "Draft an objective" }
                            }
                            span { class: "panel-index", "01 / PLAN" }
                        }
                        p { class: "panel-description", "Describe the outcome first. The next milestone connects this draft form to the gateway planning endpoint." }
                        label { "WORKLOAD OBJECTIVE" }
                        textarea {
                            value: "{objective}",
                            placeholder: "e.g. Review monthly invoices and flag arithmetic anomalies...",
                            oninput: move |event| {
                                objective.set(event.value());
                                prepared.set(false);
                            }
                        }
                        div { class: "form-footer",
                            span { "Draft only · No model call" }
                            button {
                                class: "primary-button",
                                disabled: objective().trim().is_empty(),
                                onclick: move |_| prepared.set(true),
                                "Prepare draft  →"
                            }
                        }
                        if prepared() {
                            div { class: "inline-notice",
                                "Draft captured in the UI. Gateway integration is the next implementation step."
                            }
                        }
                    }
                    div { class: "panel principles-panel",
                        div { class: "eyebrow", "RUNTIME PRINCIPLES" }
                        h3 { "Authority stays outside the model." }
                        Principle { number: "01", title: "Policy beats prompts", body: "Deterministic enforcement owns privacy and execution boundaries." }
                        Principle { number: "02", title: "No-LLM is a valid route", body: "Use ordinary software when it can satisfy the workload contract." }
                        Principle { number: "03", title: "Evidence before trust", body: "Record which checks ran; never claim universal proof of truth." }
                    }
                }
                footer { "ROCKSOUL-WX · EARLY FOUNDATION · PLANNING ONLY" }
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
