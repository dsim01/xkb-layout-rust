#![forbid(unsafe_code)]

use std::io::{self, Write as _};
use std::process::ExitCode;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xkb::{
    self, ConnectionExt as _, EventType, ID, MapPart, SelectEventsAux, StatePart,
};
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, EventMask, Window,
};
use x11rb::rust_connection::RustConnection;

const LAYOUT_FIELD: usize = 2;

const USAGE: &str = "\
Usage: xkb-layout [OPTIONS]

Prints the active XKB keyboard layout (e.g. `us`).

Options:
  -w, --watch  Keep running and print a new line every time the layout changes
  -h, --help   Print this help";

type Error = Box<dyn std::error::Error>;

fn layout_at(data: &[u8], group: usize) -> Option<&[u8]> {
    let field = data.split(|&b| b == 0).nth(LAYOUT_FIELD)?;
    field
        .split(|&b| b == b',')
        .nth(group)
        .filter(|layout| !layout.is_empty())
}

struct Xkb {
    conn: RustConnection,
    root: Window,
    rules_atom: u32,
}

impl Xkb {
    fn connect() -> Result<(Self, usize), Error> {
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|e| format!("failed open display: {e}"))?;
        let root = conn.setup().roots[screen_num].root;

        let ext = conn.xkb_use_extension(1, 0)?;
        let state = conn.xkb_get_state(ID::USE_CORE_KBD.into())?;
        let atom = conn.intern_atom(true, b"_XKB_RULES_NAMES")?;

        if !ext.reply()?.supported {
            return Err("xkb extension not supported".into());
        }
        let state = state
            .reply()
            .map_err(|e| format!("failed get xkb state: {e}"))?;
        let rules_atom = atom.reply()?.atom;
        if rules_atom == u32::from(AtomEnum::NONE) {
            return Err("failed get atom property".into());
        }

        Ok((
            Self {
                conn,
                root,
                rules_atom,
            },
            group_index(state.group),
        ))
    }

    fn rules_names(&self) -> Result<Vec<u8>, Error> {
        let prop = self
            .conn
            .get_property(false, self.root, self.rules_atom, AtomEnum::ANY, 0, 1024)?
            .reply()?;
        if prop.format != 8 {
            return Err("failed get or parse layout list".into());
        }
        Ok(prop.value)
    }

    fn subscribe(&self) -> Result<(), Error> {
        let group_changes = StatePart::GROUP_STATE;
        let details = SelectEventsAux::new().state_notify(xkb::SelectEventsAuxStateNotify {
            affect_state: group_changes,
            state_details: group_changes,
        });
        self.conn
            .xkb_select_events(
                ID::USE_CORE_KBD.into(),
                EventType::from(0u16),
                EventType::from(0u16),
                MapPart::from(0u16),
                MapPart::from(0u16),
                &details,
            )?
            .check()?;
        self.conn
            .change_window_attributes(
                self.root,
                &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
            )?
            .check()?;
        Ok(())
    }
}

fn group_index(group: xkb::Group) -> usize {
    usize::from(u8::from(group))
}

fn print_layout(rules: &[u8], group: usize) -> Result<(), Error> {
    let layout =
        layout_at(rules, group).ok_or("layout index out of bounds or layout list empty")?;
    let mut out = io::stdout().lock();
    out.write_all(layout)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

fn run_once() -> Result<(), Error> {
    let (xkb, group) = Xkb::connect()?;
    print_layout(&xkb.rules_names()?, group)
}

fn run_watch() -> Result<(), Error> {
    let (xkb, mut group) = Xkb::connect()?;
    xkb.subscribe()?;
    let mut rules = xkb.rules_names()?;
    print_layout(&rules, group)?;
    let mut last = layout_at(&rules, group).map(<[u8]>::to_vec);

    loop {
        match xkb.conn.wait_for_event()? {
            Event::XkbStateNotify(ev) => group = group_index(ev.group),
            Event::PropertyNotify(ev) if ev.window == xkb.root && ev.atom == xkb.rules_atom => {
                rules = xkb.rules_names()?;
            }
            _ => continue,
        }

        let current = layout_at(&rules, group);
        if current != last.as_deref() {
            print_layout(&rules, group)?;
            last = current.map(<[u8]>::to_vec);
        }
    }
}

fn main() -> ExitCode {
    let mut watch = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-w" | "--watch" => watch = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}\n\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }

    let result = if watch { run_watch() } else { run_once() };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e)
            if e.downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("X11: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"evdev\0pc105\0us,ua\0,\0grp:alt_shift_toggle\0";

    #[test]
    fn picks_layout_by_group() {
        assert_eq!(layout_at(DATA, 0), Some(&b"us"[..]));
        assert_eq!(layout_at(DATA, 1), Some(&b"ua"[..]));
    }

    #[test]
    fn single_layout_without_trailing_nul() {
        assert_eq!(layout_at(b"evdev\0pc105\0us", 0), Some(&b"us"[..]));
    }

    #[test]
    fn group_out_of_bounds() {
        assert_eq!(layout_at(DATA, 2), None);
    }

    #[test]
    fn rejects_missing_or_empty_field() {
        assert_eq!(layout_at(b"evdev\0pc105", 0), None);
        assert_eq!(layout_at(b"evdev\0pc105\0\0", 0), None);
        assert_eq!(layout_at(b"", 0), None);
    }
}
