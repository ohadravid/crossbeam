//! Multi-producer multi-consumer channels for message passing.
//!
//! Channels are concurrent FIFO queues used for passing messages between threads.
//!
//! Crossbeam's channels are an alternative to the [`std::sync::mpsc`] channels provided by the
//! standard library. They are an improvement in pretty much all aspects: ergonomics, flexibility,
//! features, performance.
//!
//! # Types of channels
//!
//! A channel can be constructed by calling functions [`unbounded`] and [`bounded`]. The former
//! creates a channel of unbounded capacity (i.e. it can contain an arbitrary number of messages),
//! while the latter creates a channel of bounded capacity (i.e. there is a limit to how many
//! messages it can hold at a time).
//!
//! Both constructors returns a pair of two values: a sender and a receiver. Senders and receivers
//! represent two opposite sides of a channel. Messages are sent using senders and received using
//! receivers.
//!
//! Creating an unbounded channel:
//!
//! ```
//! use crossbeam_channel::unbounded;
//!
//! // Create an unbounded channel.
//! let (s, r) = unbounded();
//!
//! // Can send an arbitrarily large number of messages.
//! for i in 0..1000 {
//!     s.send(i);
//! }
//! ```
//!
//! Creating a bounded channel:
//!
//! ```
//! use crossbeam_channel::bounded;
//!
//! // Create a channel that can hold at most 5 messages at a time.
//! let (s, r) = bounded(5);
//!
//! // Can send only 5 messages.
//! for i in 0..5 {
//!     s.send(i);
//! }
//!
//! // An attempt to send one more message will fail.
//! // assert!(s.try_send(5).is_some());
//! ```
//!
//! An interesting special case is a bounded, zero-capacity channel. This kind of channel cannot
//! hold any messages at all! In order to send a message through the channel, another thread must
//! be waiting at the other end of it at the same time:
//!
//! ```
//! use crossbeam_channel::bounded;
//!
//! use std::thread;
//!
//! // Create a zero-capacity channel.
//! let (s, r) = bounded(0);
//!
//! // Spawn a thread that sends a message into the channel.
//! thread::spawn(move || s.send("Hi!"));
//!
//! // Receive the message.
//! assert_eq!(r.recv(), Some("Hi!"));
//! ```
//!
//! # Sharing channels
//!
//! Senders and receivers can be either shared by reference or cloned and then sent to other
//! threads. Feel free to use any of these two approaches as you like.
//!
//! Sharing by reference:
//!
//! ```
//! extern crate crossbeam_channel;
//! extern crate crossbeam_utils;
//! # fn main() {
//!
//! use crossbeam_channel::unbounded;
//!
//! let (s, r) = unbounded();
//!
//! crossbeam_utils::scoped::scope(|scope| {
//!     // Spawn a thread that sends one message and then receives one.
//!     scope.spawn(|| {
//!         s.send(1);
//!         r.recv().unwrap();
//!     });
//!
//!     // Spawn another thread that does the same thing.
//!     // Both closures capture `s` and `r` by reference.
//!     scope.spawn(|| {
//!         s.send(2);
//!         r.recv().unwrap();
//!     });
//! });
//!
//! # }
//! ```
//!
//! Sharing by sending:
//!
//! ```
//! use std::thread;
//! use crossbeam_channel::unbounded;
//!
//! let (s, r) = unbounded();
//! let (s2, r2) = (s.clone(), r.clone());
//!
//! // Spawn a thread that sends one message and then receives one.
//! // Here, `s` and `r` are moved into the closure (sent into the thread).
//! thread::spawn(move || {
//!     s.send(1);
//!     r.recv().unwrap();
//! });
//!
//! // Spawn another thread that does the same thing.
//! // Here, `s2` and `r2` are moved into the closure (sent into the thread).
//! thread::spawn(move || {
//!     s2.send(2);
//!     r2.recv().unwrap();
//! });
//! ```
//!
//! # Closing
//!
//! As soon as all senders or all receivers associated with a channel are dropped, it becomes
//! closed. Messages cannot be sent into a closed channel anymore, but the remaining
//! messages can still be received.
//!
//! ```
//! use crossbeam_channel::unbounded;
//!
//! let (s, r) = unbounded::<&str>();
//!
//! // The only receiver is dropped, closing the channel.
//! drop(r);
//!
//! // Attempting to send a message will result in an error.
//! // assert_eq!(s.try_send("hello"), Err(TrySendError::Closed("hello")));
//! ```
//!
//! ```
//! use crossbeam_channel::unbounded;
//!
//! let (s, r) = unbounded();
//! s.send(1);
//! s.send(2);
//! s.send(3);
//!
//! // The only sender is dropped, closing the channel.
//! drop(s);
//!
//! // The remaining messages can be received.
//! assert_eq!(r.try_recv(), Some(1));
//! assert_eq!(r.try_recv(), Some(2));
//! assert_eq!(r.try_recv(), Some(3));
//!
//! // However, attempting to receive another message will result in an error.
//! assert_eq!(r.try_recv(), None);
//! ```
//!
//! # Blocking and non-blocking operations
//!
//! Send and receive operations come in three variants:
//!
//! 1. Non-blocking: [`try_recv`].
//! 2. Blocking: [`send`] and [`recv`].
//! 3. Blocking with a timeout: [`send_timeout`] and [`recv_timeout`].
//!
//! The non-blocking variant attempts to perform the operation, but doesn't block the current
//! thread on failure (e.g. if receiving a message from an empty channel).
//!
//! The blocking variant will wait until the operation can be performed or the channel gets closed.
//!
//! Blocking with a timeout does the same thing, but blocks the current thread only for a limited
//! amount time.
//!
//! # Iteration
//!
//! Receivers can be turned into iterators. For example, calling [`iter`] creates an iterator that
//! returns messages until the channel is closed. Note that iteration may block while waiting
//! for the next message.
//!
//! ```
//! use crossbeam_channel::unbounded;
//!
//! let (s, r) = unbounded();
//! s.send(1);
//! s.send(2);
//! s.send(3);
//!
//! // Drop the sender in order to close the channel.
//! drop(s);
//!
//! // Receive all remaining messages.
//! let v: Vec<_> = r.collect();
//! assert_eq!(v, [1, 2, 3]);
//! ```
//!
//! # Selection
//!
//! Selection allows you to declare a set of operations on channels and perform exactly one of
//! them, whichever becomes ready first, possibly blocking until that happens.
//!
//! For example, selection can be used to receive a message from one of the two channels, blocking
//! until a message appears on either of them:
//!
//! ```
//! # #[macro_use]
//! # extern crate crossbeam_channel;
//! # fn main() {
//!
//! /*
//! use std::thread;
//! use crossbeam_channel::unbounded;
//!
//! let (s1, r1) = unbounded();
//! let (s2, r2) = unbounded();
//!
//! thread::spawn(move || s1.send("foo"));
//! thread::spawn(move || s2.send("bar"));
//!
//! select_loop! {
//!     recv(r1, msg) => {
//!         println!("Received a message from the first channel: {}", msg);
//!     }
//!     recv(r2, msg) => {
//!         println!("Received a message from the second channel: {}", msg);
//!     }
//! }
//! */
//!
//! # }
//! ```
//!
//! The syntax of [`select_loop!`] is very similar to the one used by `match`.
//!
//! Here is another, more complicated example of selection. Here we are selecting over two
//! operations on the opposite ends of the same channel: a send and a receive operation.
//!
//! ```
//! # #[macro_use]
//! # extern crate crossbeam_channel;
//! # fn main() {
//!
//! /*
//! use crossbeam_channel::{bounded, Sender, Receiver, Select};
//! use std::thread;
//!
//! // Either send my name into the channel or receive someone else's, whatever happens first.
//! fn seek<'a>(name: &'a str, s: Sender<&'a str>, r: Receiver<&'a str>) {
//!     select_loop! {
//!         recv(r, peer) => println!("{} received a message from {}.", name, peer),
//!         send(s, name) => {},
//!     }
//! }
//!
//! let (s, r) = bounded(1); // Make room for one unmatched send.
//!
//! // Pair up five people by exchanging messages over the channel.
//! // Since there is an odd number of them, one person won't have its match.
//! ["Anna", "Bob", "Cody", "Dave", "Eva"].iter()
//!     .map(|name| {
//!         let s = s.clone();
//!         let r = r.clone();
//!         thread::spawn(move || seek(name, s, r))
//!     })
//!     .collect::<Vec<_>>()
//!     .into_iter()
//!     .for_each(|t| t.join().unwrap());
//!
//! // Let's send a message to the remaining person who doesn't have a match.
//! if let Some(name) = r.try_recv() {
//!     println!("No one received {}’s message.", name);
//! }
//! */
//!
//! # }
//! ```
//!
//! For more details, take a look at the documentation of [`select_loop!`].
//!
//! If you need a more powerful interface that allows selecting over a dynamic set of channel
//! operations, use [`Select`].
//!
//! [`std::sync::mpsc`]: https://doc.rust-lang.org/std/sync/mpsc/index.html
//! [`unbounded`]: fn.unbounded.html
//! [`bounded`]: fn.bounded.html
//! [`send`]: struct.Sender.html#method.send
//! [`send_timeout`]: struct.Sender.html#method.send_timeout
//! [`try_recv`]: struct.Receiver.html#method.try_recv
//! [`recv`]: struct.Receiver.html#method.recv
//! [`recv_timeout`]: struct.Receiver.html#method.recv_timeout
//! [`select_loop!`]: macro.select_loop.html
//! [`Select`]: struct.Select.html

#![cfg_attr(feature = "nightly", feature(spin_loop_hint))]

#![warn(missing_docs, missing_debug_implementations)]

// TODO: Reexport hidden stuff through _internal module

extern crate crossbeam_epoch;
extern crate crossbeam_utils;
extern crate parking_lot;
#[doc(hidden)]
pub extern crate smallvec;

#[doc(hidden)]
#[macro_use]
pub mod select;

#[doc(hidden)]
pub mod channel;
mod flavors;
mod monitor;
#[doc(hidden)]
pub mod utils;

pub use channel::{bounded, unbounded};
pub use channel::{Receiver, Sender};
