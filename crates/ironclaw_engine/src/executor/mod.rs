//! Step execution.
//!
//! - [`ExecutionLoop`] — core loop replacing `run_agentic_loop()`
//! - [`structured`] — Tier 0 action execution (structured tool calls)
//! - [`context`] — context building for LLM calls
//! - [`intent`] — tool intent nudge detection

pub mod context;
pub mod loop_engine;
pub mod orchestrator;
pub mod planner;
pub mod prompt;
pub mod scripting;
pub mod structured;
pub(crate) mod thread_context;
pub mod tier0_prompt;
pub mod token_guard;
pub mod trace;

pub use loop_engine::ExecutionLoop;
pub use scripting::validate_python_syntax;
