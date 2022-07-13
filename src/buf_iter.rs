use std::{borrow::Cow, collections::VecDeque};

use ringbuf::{Consumer, Producer, RingBuffer};

pub fn ring_buf_queue<const BUF_SIZE: usize, T>(
) -> (RingBufQProducer<T>, RingBufQConsumer<BUF_SIZE, T>)
where
    T: Default + Copy,
{
    let (prod, cons) = RingBuffer::new(BUF_SIZE).split();

    let prod = RingBufQProducer {
        q: VecDeque::new(),
        prod,
    };
    let cons = RingBufQConsumer {
        buffer: [T::default(); BUF_SIZE],
        q: VecDeque::new(),
        cons,
    };
    (prod, cons)
}

pub struct RingBufQProducer<T> {
    q: VecDeque<Box<[T]>>,
    prod: Producer<T>,
}

impl<T> RingBufQProducer<T>
where
    T: Copy,
{
    pub fn flush(&mut self) {
        while let Some(first) = self.q.pop_front() {
            let ret = self.prod.push_slice(&first);
            if ret == 0 {
                self.q.push_front(first);
                break;
            } else if ret < first.len() {
                self.q.push_front(first[ret..].into());
                break;
            }
        }
    }

    pub fn push<E>(&mut self, elems: E)
    where
        E: AsRef<[T]> + Into<Box<[T]>>,
    {
        self.flush();
        if elems.as_ref().len() == 0 {
            return;
        }
        if self.q.is_empty() {
            let ret = self.prod.push_slice(elems.as_ref());
            if ret < elems.as_ref().len() {
                self.q.push_back(elems.as_ref()[ret..].into())
            }
        } else {
            self.q.push_back(elems.into())
        }
    }
}

pub struct RingBufQConsumer<const BUF_SIZE: usize, T> {
    buffer: [T; BUF_SIZE],
    q: VecDeque<Box<[T]>>,
    cons: Consumer<T>,
}

impl<const BUF_SIZE: usize, T> RingBufQConsumer<BUF_SIZE, T>
where
    T: Copy,
{
    pub fn pop(&mut self) -> Cow<'_, [T]> {
        let elems = &mut self.buffer;
        let ret = self.cons.pop_slice(elems);

        match self.q.pop_front() {
            Some(first) => {
                if ret > 0 {
                    self.q.push_back(elems[..ret].into());
                }
                Cow::Owned(first.to_vec())
            }
            None => Cow::Borrowed(&elems[..ret]),
        }
    }
}
