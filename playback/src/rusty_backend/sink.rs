use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use super::stream::{OutputStreamHandle, PlayError};
use super::{queue, source::Done, Sample, Source};
use crate::PlayerCmd;
use cpal::FromSample;

/// Handle to an device that outputs sounds.
///
/// Dropping the `Sink` stops all sounds. You can use `detach` if you want the sounds to continue
/// playing.
pub struct Sink {
    queue_tx: Arc<queue::SourcesQueueInput<f32>>,
    sleep_until_end: Mutex<Option<Receiver<()>>>,

    controls: Arc<Controls>,
    sound_count: Arc<AtomicUsize>,

    detached: bool,
    elapsed: Arc<RwLock<Duration>>,
    message_tx: Sender<PlayerCmd>,
}

struct Controls {
    pause: AtomicBool,
    volume: Mutex<f32>,
    seek: Mutex<Option<Duration>>,
    stopped: AtomicBool,
    speed: Mutex<f32>,
    to_clear: Mutex<u32>,
    do_skip: AtomicBool,
}

impl Sink {
    /// Builds a new `Sink`, beginning playback on a stream.
    #[inline]
    pub fn try_new(stream: &OutputStreamHandle, tx: Sender<PlayerCmd>) -> Result<Self, PlayError> {
        let (sink, queue_rx) = Self::new_idle(tx);
        stream.play_raw(queue_rx)?;
        Ok(sink)
    }
    /// Builds a new `Sink`.
    #[inline]
    pub fn new_idle(tx: Sender<PlayerCmd>) -> (Self, queue::SourcesQueueOutput<f32>) {
        // pub fn new_idle() -> (Sink, queue::SourcesQueueOutput<f32>) {
        // let (queue_tx, queue_rx) = queue::queue(true);
        let (queue_tx, queue_rx) = queue::queue(true);

        let sink = Sink {
            queue_tx,
            sleep_until_end: Mutex::new(None),
            controls: Arc::new(Controls {
                pause: AtomicBool::new(false),
                volume: Mutex::new(1.0),
                stopped: AtomicBool::new(false),
                seek: Mutex::new(None),
                speed: Mutex::new(1.0),
                to_clear: Mutex::new(0),
                do_skip: AtomicBool::new(false),
            }),
            sound_count: Arc::new(AtomicUsize::new(0)),
            detached: false,
            elapsed: Arc::new(RwLock::new(Duration::from_secs(0))),
            message_tx: tx,
        };
        (sink, queue_rx)
    }

    /// Appends a sound to the queue of sounds to play.
    #[inline]
    pub fn append<S>(&self, source: S)
    where
        S: Source + Send + 'static,
        f32: FromSample<S::Item>,
        S::Item: Sample + Send,
    {
        // Wait for queue to flush then resume stopped playback
        if self.controls.stopped.load(Ordering::SeqCst) {
            if self.sound_count.load(Ordering::SeqCst) > 0 {
                self.sleep_until_end();
            }
            self.controls.stopped.store(false, Ordering::SeqCst);
        }

        let controls = self.controls.clone();

        let start_played = AtomicBool::new(false);

        let elapsed = self.elapsed.clone();
        let tx = self.message_tx.clone();
        let source = source
            .speed(1.0)
            .pausable(false)
            .amplify(1.0)
            .skippable()
            .stoppable()
            .periodic_access(Duration::from_millis(100), move |src| {
                let position = src.elapsed().as_secs() as i64;
                tx.send(PlayerCmd::Progress(position)).ok();
            })
            // .periodic_access(Duration::from_millis(50), move |src| {
            //     let mut src = src.inner_mut();
            //     if controls.stopped.load(Ordering::SeqCst) {
            //         src.stop();
            //     } else if controls.do_skip.load(Ordering::SeqCst) {
            //         src.inner_mut().skip();
            //         controls.do_skip.store(false, Ordering::SeqCst);
            //     } else {
            //         if let Some(seek_time) = controls.seek.lock().unwrap().take() {
            //             src.seek(seek_time).unwrap();
            //         }
            //         *elapsed.write().unwrap() = src.elapsed();
            //         let mut new_factor = *controls.volume.lock().unwrap();
            //         if new_factor < 0.0001 {
            //             new_factor = 0.0001;
            //         }
            //         src.inner_mut().inner_mut().set_factor(new_factor);
            //         src.inner_mut()
            //             .inner_mut()
            //             .inner_mut()
            //             .set_paused(controls.pause.load(Ordering::SeqCst));
            //         src.inner_mut()
            //             .inner_mut()
            //             .inner_mut()
            //             .inner_mut()
            //             .set_factor(*controls.speed.lock().unwrap());
            //     }
            // })
            .periodic_access(Duration::from_millis(5), move |src| {
                let src = src.inner_mut();
                if controls.stopped.load(Ordering::SeqCst) {
                    src.stop();
                } else if controls.do_skip.load(Ordering::SeqCst) {
                    src.inner_mut().skip();
                    controls.do_skip.store(false, Ordering::SeqCst);
                } else {
                    if let Some(seek_time) = controls.seek.lock().unwrap().take() {
                        src.seek(seek_time).unwrap();
                    }
                    *elapsed.write().unwrap() = src.elapsed();
                    {
                        let mut to_clear = controls.to_clear.lock().unwrap();
                        if *to_clear > 0 {
                            let _ = src.inner_mut().skip();
                            *to_clear -= 1;
                        }
                    }
                    let amp = src.inner_mut().inner_mut();
                    amp.set_factor(*controls.volume.lock().unwrap());
                    amp.inner_mut()
                        .set_paused(controls.pause.load(Ordering::SeqCst));
                    amp.inner_mut()
                        .inner_mut()
                        .set_factor(*controls.speed.lock().unwrap());
                    start_played.store(true, Ordering::SeqCst);
                }
            })
            .convert_samples();
        self.sound_count.fetch_add(1, Ordering::Relaxed);
        let source = Done::new(source, self.sound_count.clone());
        *self.sleep_until_end.lock().unwrap() = Some(self.queue_tx.append_with_signal(source));
    }

    /// Gets the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn volume(&self) -> f32 {
        *self.controls.volume.lock().unwrap()
    }

    /// Changes the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than `1.0` will
    /// multiply each sample by this value.
    #[inline]
    pub fn set_volume(&self, value: f32) {
        *self.controls.volume.lock().unwrap() = value;
    }

    /// Gets the speed of the sound.
    ///
    /// The value `1.0` is the "normal" speed (unfiltered input). Any value other than `1.0` will
    /// change the play speed of the sound.
    #[inline]
    pub fn speed(&self) -> f32 {
        *self.controls.speed.lock().unwrap()
    }

    /// Changes the speed of the sound.
    ///
    /// The value `1.0` is the "normal" speed (unfiltered input). Any value other than `1.0` will
    /// change the play speed of the sound.
    #[inline]
    pub fn set_speed(&self, value: f32) {
        *self.controls.speed.lock().unwrap() = value;
    }

    /// Resumes playback of a paused sink.
    ///
    /// No effect if not paused.
    #[inline]
    pub fn play(&self) {
        self.controls.pause.store(false, Ordering::SeqCst);
    }

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    ///
    /// A paused sink can be resumed with `play()`.
    pub fn pause(&self) {
        self.controls.pause.store(true, Ordering::SeqCst);
    }

    /// Gets if a sink is paused
    ///
    /// Sinks can be paused and resumed using `pause()` and `play()`. This returns `true` if the
    /// sink is paused.
    pub fn is_paused(&self) -> bool {
        self.controls.pause.load(Ordering::SeqCst)
    }

    pub fn seek(&self, seek_time: Duration) {
        if self.is_paused() {
            self.play();
        }
        *self.controls.seek.lock().unwrap() = Some(seek_time);
    }
    /// Toggles playback of the sink
    pub fn toggle_playback(&self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }
    /// Removes all currently loaded `Source`s from the `Sink`, and pauses it.
    ///
    /// See `pause()` for information about pausing a `Sink`.
    pub fn clear(&self) {
        let len = self.sound_count.load(Ordering::SeqCst) as u32;
        *self.controls.to_clear.lock().unwrap() = len;
        self.sleep_until_end();
        self.pause();
    }

    /// Skips to the next `Source` in the `Sink`
    ///
    /// If there are more `Source`s appended to the `Sink` at the time,
    /// it will play the next one. Otherwise, the `Sink` will finish as if
    /// it had finished playing a `Source` all the way through.
    pub fn skip_one(&self) {
        let len = self.sound_count.load(Ordering::SeqCst) as u32;
        let mut to_clear = self.controls.to_clear.lock().unwrap();
        if len > *to_clear {
            *to_clear += 1;
        }
    }

    /// Stops the sink by emptying the queue.
    #[inline]
    pub fn stop(&self) {
        self.controls.stopped.store(true, Ordering::SeqCst);
    }

    /// Destroys the sink without stopping the sounds that are still playing.
    #[inline]
    pub fn detach(mut self) {
        self.detached = true;
    }

    /// Sleeps the current thread until the sound ends.
    #[inline]
    pub fn sleep_until_end(&self) {
        if let Some(sleep_until_end) = self.sleep_until_end.lock().unwrap().take() {
            let _ = sleep_until_end.recv();
        }
    }

    /// Returns true if this sink has no more sounds to play.
    #[inline]
    pub fn empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of sounds currently in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.sound_count.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        *self.elapsed.read().unwrap()
    }

    // Spawns a new thread to sleep until the sound ends, and then sends the SoundEnded
    // message through the given Sender.
    pub fn message_on_end(&self) {
        // let tx1 = Sender::clone(&self.message_tx);
        // let tx1 = self.message_tx.clone();
        if let Some(sleep_until_end) = self.sleep_until_end.lock().unwrap().take() {
            std::thread::spawn(move || {
                let _drop = sleep_until_end.recv();
                // tx1.send(PlayerMsg::Eos).ok();
                if let Err(e) = crate::audio_cmd::<()>(PlayerCmd::Eos, true) {
                    debug!("Error in message_on_end: {e}");
                }
                // if let Err(e) = tx1.send(PlayerMsg::Eos) {
                //     eprintln!("Error is: {}", e);
                // }
            });
        }
    }
}

impl Drop for Sink {
    #[inline]
    fn drop(&mut self) {
        self.queue_tx.set_keep_alive_if_empty(false);

        if !self.detached {
            self.controls.stopped.store(true, Ordering::Relaxed);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::buffer::SamplesBuffer;
//     use super::{Sink, Source};
//     use std::sync::atomic::Ordering;

//     #[test]
//     fn test_pause_and_stop() {
//         let (sink, mut queue_rx) = Sink::new_idle();

//         // assert_eq!(queue_rx.next(), Some(0.0));

//         let v = vec![10i16, -10, 20, -20, 30, -30];

//         // Low rate to ensure immediate control.
//         sink.append(SamplesBuffer::new(1, 1, v.clone()));
//         let mut src = SamplesBuffer::new(1, 1, v).convert_samples();

//         assert_eq!(queue_rx.next(), src.next());
//         assert_eq!(queue_rx.next(), src.next());

//         sink.pause();

//         assert_eq!(queue_rx.next(), Some(0.0));

//         sink.play();

//         assert_eq!(queue_rx.next(), src.next());
//         assert_eq!(queue_rx.next(), src.next());

//         sink.stop();

//         assert_eq!(queue_rx.next(), Some(0.0));

//         assert_eq!(sink.empty(), true);
//     }

//     #[test]
//     fn test_stop_and_start() {
//         let (sink, mut queue_rx) = Sink::new_idle();

//         let v = vec![10i16, -10, 20, -20, 30, -30];

//         sink.append(SamplesBuffer::new(1, 1, v.clone()));
//         let mut src = SamplesBuffer::new(1, 1, v.clone()).convert_samples();

//         assert_eq!(queue_rx.next(), src.next());
//         assert_eq!(queue_rx.next(), src.next());

//         sink.stop();

//         assert!(sink.controls.stopped.load(Ordering::SeqCst));
//         assert_eq!(queue_rx.next(), Some(0.0));

//         src = SamplesBuffer::new(1, 1, v.clone()).convert_samples();
//         sink.append(SamplesBuffer::new(1, 1, v));

//         assert!(!sink.controls.stopped.load(Ordering::SeqCst));
//         // Flush silence
//         let mut queue_rx = queue_rx.skip_while(|v| *v == 0.0);

//         assert_eq!(queue_rx.next(), src.next());
//         assert_eq!(queue_rx.next(), src.next());
//     }

//     #[test]
//     fn test_volume() {
//         let (sink, mut queue_rx) = Sink::new_idle();

//         let v = vec![10i16, -10, 20, -20, 30, -30];

//         // High rate to avoid immediate control.
//         sink.append(SamplesBuffer::new(2, 44100, v.clone()));
//         let src = SamplesBuffer::new(2, 44100, v.clone()).convert_samples();

//         let mut src = src.amplify(0.5);
//         sink.set_volume(0.5);

//         for _ in 0..v.len() {
//             assert_eq!(queue_rx.next(), src.next());
//         }
//     }
// }