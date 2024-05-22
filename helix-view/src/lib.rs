#[macro_use]
pub mod macros;

pub mod base64;
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod events;
pub mod graphics;
pub mod gutter;
pub mod handlers;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register;
pub mod theme;
pub mod tree;
pub mod view;

use std::num::NonZeroUsize;

// uses NonZeroUsize so Option<DocumentId> use a byte rather than two
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentId(NonZeroUsize);

impl Default for DocumentId {
    fn default() -> DocumentId {
        // Safety: 1 is non-zero
        DocumentId(unsafe { NonZeroUsize::new_unchecked(1) })
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

slotmap::new_key_type! {
    pub struct ViewId;
}

pub enum Align {
    Top,
    Center,
    Bottom,
}

pub fn align_view(doc: &Document, view: &mut View, align: Align) {
    let doc_text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(doc_text);
    let viewport = view.inner_area(doc);
    let last_line_height = viewport.height.saturating_sub(1);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    let text_fmt = doc.text_format(viewport.width, None);
    let annotations = view.text_annotations(doc, None);
    (view.offset.anchor, view.offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        cursor,
        -(relative as isize),
        0,
        &text_fmt,
        &annotations,
    );
}

pub use document::Document;
pub use editor::Editor;
use helix_core::char_idx_at_visual_offset;
pub use theme::Theme;
pub use view::View;

// -- TudbuT mod begin

/// communicates with the outside world
#[allow(non_camel_case_types)]
struct TT__Server {
    tcp_listener: std::net::TcpListener,
    tcp_streams: Vec<std::net::TcpStream>,
    last_word: String,
}

impl TT__Server {
    fn new() -> Self {
        // god damn
        #[derive(Clone)]
        struct AddrIter<T: Iterator<Item = u16> + Clone> {
            inner: T,
        }
        impl<T: Iterator<Item = u16> + Clone> AddrIter<T> {
            pub fn new(inner: T) -> Self {
                Self { inner }
            }
        }
        impl<T: Iterator<Item = u16> + Clone> Iterator for AddrIter<T> {
            type Item = std::net::SocketAddr;

            fn next(&mut self) -> Option<Self::Item> {
                Some(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::new(127, 0, 0, 1),
                    self.inner.next()?,
                )))
            }
        }
        impl<T: Iterator<Item = u16> + Clone> std::net::ToSocketAddrs for AddrIter<T> {
            type Iter = Self;

            fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
                Ok(self.clone())
            }
        }

        let me = Self {
            tcp_listener: std::net::TcpListener::bind(AddrIter::new(7400..)).expect("DO NOT REPORT THIS TO HELIX!!! This is a TudbuT mod error: unable to start TT__Server"),
            tcp_streams: Vec::new(),
            last_word: String::default(),
        };

        me.tcp_listener.set_nonblocking(true).expect(
            "DO NOT REPORT THIS TO HELIX!!! This is a TudbuT mod error: unable to set nonblocking",
        );

        me
    }

    fn update(&mut self, mut new_word: String) {
        use std::io::{ErrorKind, Read, Write};

        if let Ok((x, _)) = self.tcp_listener.accept() {
            if x.set_nonblocking(true).is_ok() {
                self.tcp_streams.push(x);
            }
        }
        new_word += "\n";
        if self.last_word != new_word {
            let b = new_word.as_bytes().len();
            let mut buf = [0u8; 1024];
            let mut to_remove = Vec::new();
            for (i, stream) in self.tcp_streams.iter_mut().enumerate() {
                match stream.read(&mut buf) {
                    Ok(_n @ 1..) => (),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => (),
                    _ => {
                        to_remove.push(i);
                        continue;
                    }
                }
                match stream.write(new_word.as_bytes()) {
                    Ok(n) if n == b => (),
                    _ => to_remove.push(i),
                }
            }
            for i in to_remove {
                let stream = self.tcp_streams.remove(i);
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }

            self.last_word = new_word;
        }
    }
}

#[allow(non_upper_case_globals)]
static TT__SERVER: std::sync::Mutex<Option<TT__Server>> = std::sync::Mutex::new(None);

#[allow(non_snake_case)]
pub fn tt__update_manpage(doc: &Document, view_id: ViewId) {
    let c = helix_core::syntax::FileType::Extension("c".to_owned());

    fn get_word(text: &str, idx: usize) -> &str {
        let mut start = idx;
        let mut end = idx;

        const WORD_END: &str = " {}()[],#+-/*;<>&!\"'%$?~^|'\n\r";

        while start != 0 && !WORD_END.contains(text.get(start - 1..start).unwrap_or("\0")) {
            start = match start.checked_sub(1) {
                Some(n) => n,
                None => break,
            }
        }

        while end != text.len() && !WORD_END.contains(text.get(end..=end).unwrap_or("\0")) {
            end += 1;
        }

        &text[start..end]
    }

    if doc
        .language
        .as_ref()
        .is_some_and(|x| x.file_types.contains(&c))
    {
        let idx = doc.selection(view_id).primary().to() - 1;
        let lidx = doc.text().char_to_line(idx);
        let line_idx = doc.text().line_to_char(lidx);
        let next_line_idx = doc.text().line_to_char(lidx + 1);
        let idx_in_line = idx - line_idx;

        let text_in_line = doc.text().slice(line_idx..next_line_idx).to_string();

        let word = get_word(&text_in_line, idx_in_line);

        let mut server = TT__SERVER
            .lock()
            .expect("DO NOT REPORT THIS TO HELIX (mod error)");
        if let Some(ref mut server) = *server {
            server.update(word.to_owned());
        } else {
            *server = Some(TT__Server::new());
        }
    }
}

// -- TudbuT mod end
