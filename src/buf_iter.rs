use std::{collections::VecDeque, borrow::Cow};

pub struct LineSplitProducer<const SIZE: usize> {
    last_char: char,
    buffer: [u8; SIZE],
    capacity: usize,
}

impl<const SIZE: usize> LineSplitProducer<SIZE> {
    pub fn new() -> Self {
        Self {
            last_char: '0',
            buffer: [0u8; SIZE],
            capacity: 0,
        }
    }
    pub fn push<'a, 'b: 'a>(&'b mut self, buf: &'a str) -> Vec<Cow<'a, str>> {
        let mut lines = vec![];
        let mut s = 0;

        for (i, c) in buf.char_indices() {
            let last_char = self.last_char;
            self.last_char = c;

            if let '\r' | '\n' = c {
                if let '\r' | '\n' = last_char {
                    if last_char == c {
                        lines.push(Cow::Borrowed(""))
                    } else {
                        s = i + c.len_utf8();
                    }
                } else if self.capacity == 0 {
                    lines.push(Cow::Borrowed(&buf[s..i]));
                }
            } else if SIZE < self.capacity + (i - s) {
                
            }
        }

        lines
    }

    // pub fn next(&mut self) -> BufIterRes<'a> {
    //     if buf.is_empty() {
    //         return BufIterRes::End;
    //     }

    //     let mut tmp = buf;
    //     buf = "";

    //     for (i, c) in tmp.char_indices() {
    //         let last_char = self.last_char;
    //         self.last_char = c;

    //         if let '\r' | '\n' = c {
    //             buf = &tmp[i + c.len_utf8()..];
    //             if let '\r' | '\n' = last_char {
    //                 if last_char == c {
    //                     return BufIterRes::Line("");
    //                 } else {
    //                     tmp = buf;
    //                 }
    //             } else {
    //                 return BufIterRes::Line(&tmp[..i]);
    //             }
    //         }
    //     }

    //     BufIterRes::Partial(tmp)
    // }
}

#[cfg(test)]
mod test {}
