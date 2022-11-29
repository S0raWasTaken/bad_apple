use std::sync::mpsc::{channel, Receiver, RecvError, SendError, Sender};

pub struct BiChannel<A, B>(Sender<A>, Receiver<B>);

impl<A, B> BiChannel<A, B> {
    pub fn new() -> (BiChannel<A, B>, BiChannel<B, A>) {
        let (txa, rxa) = channel();
        let (txb, rxb) = channel();
        (BiChannel::<A, B>(txa, rxb), BiChannel::<B, A>(txb, rxa))
    }

    #[inline]
    pub fn recv(&self) -> Result<B, RecvError> {
        self.1.recv()
    }

    #[inline]
    pub fn send(&self, val: A) -> Result<(), SendError<A>> {
        self.0.send(val)
    }

    pub fn send_recv(&self, val: A) -> Option<B> {
        self.send(val).ok()?;
        self.recv().ok()
    }
}
