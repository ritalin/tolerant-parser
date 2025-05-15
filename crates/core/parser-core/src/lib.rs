pub mod event_dispatcher;


#[derive(PartialEq, Debug, thiserror::Error)]
pub enum ParseError {
    /// no more entry in state stack
    #[error("no more entry in state stack (context: {context})")]
    NoMoreState { context: String },
    /// no more entry in state stack
    #[error("no more entry in goto candidate (state: {state}, lhs: {lhs})")]
    NoGotoCandidate {state: usize, lhs: String},
    /// request to recover parsing state
    #[error("request to recover parsing state")]
    RequestRecovery,
    /// unmatch accept state
    #[error("unmatch accept state")]
    NotAccept,

}
