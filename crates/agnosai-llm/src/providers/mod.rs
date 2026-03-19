// LLM provider implementations — each is direct HTTP via reqwest.

pub mod anthropic;
pub mod ollama;
pub mod openai;

// TODO: Implement remaining providers in later phases
// pub mod gemini;
// pub mod deepseek;
// pub mod mistral;
// pub mod groq;
// pub mod lmstudio;
// pub mod hoosh;
