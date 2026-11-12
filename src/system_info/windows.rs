mod audio;
mod battery;
mod disk;
mod graphics;
mod mainboard;
mod memory;
mod monitor;
mod network;
mod pnp;
mod processor;
mod system;

use super::{
    mi::{Client, Record},
    model::{HardwareDevice, HardwareKind, Snapshot},
};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) use system::uptime;

struct Reader<'a> {
    client: Client,
    cancelled: &'a AtomicBool,
    errors: Vec<String>,
}

impl Reader<'_> {
    fn query(
        &mut self,
        namespace: &str,
        class: &str,
        fields: &'static [&'static str],
        filter: Option<&str>,
    ) -> Vec<Record> {
        match self
            .client
            .query(namespace, class, fields, filter, 5, self.cancelled)
        {
            Ok(records) => records,
            Err(error) => {
                if !self.cancelled.load(Ordering::Relaxed) {
                    self.errors.push(error);
                }
                Vec::new()
            }
        }
    }
}

pub(super) fn read(cancelled: &AtomicBool) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let client = match Client::new() {
        Ok(client) => client,
        Err(error) => {
            snapshot.errors.push(error);
            return snapshot;
        }
    };
    let mut reader = Reader {
        client,
        cancelled,
        errors: Vec::new(),
    };
    system::read(&mut reader, &mut snapshot);
    snapshot.add(HardwareKind::Mainboard, mainboard::read(&mut reader));
    snapshot.add(HardwareKind::Processor, processor::read(&mut reader));
    snapshot.add(HardwareKind::Memory, memory::read(&mut reader));
    snapshot.add(HardwareKind::Graphics, graphics::read(&mut reader));
    snapshot.add(HardwareKind::Monitor, monitor::read(&mut reader));
    snapshot.add(HardwareKind::Disk, disk::read(&mut reader));
    let batteries = battery::read(&mut reader);
    if !batteries.is_empty() {
        snapshot.add(HardwareKind::Battery, batteries);
    }
    snapshot.add(
        HardwareKind::Audio,
        audio::read(cancelled, &mut reader.errors),
    );
    snapshot.add(HardwareKind::Network, network::read(&mut reader));
    snapshot.errors = reader.errors;
    snapshot
}

fn utf16(value: &[u16]) -> String {
    String::from_utf16_lossy(value.split(|&c| c == 0).next().unwrap_or_default())
        .trim()
        .to_owned()
}

fn joined(item: &Record, fields: &[&str]) -> String {
    fields
        .iter()
        .filter_map(|field| item.text(field))
        .collect::<Vec<_>>()
        .join(" ")
}

fn positive(value: Option<u64>) -> Option<u64> {
    value.filter(|&value| value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "queries this Windows machine through MI and DXGI"]
    fn local_system_information() {
        let snapshot = read(&AtomicBool::new(false));
        eprintln!("{}", snapshot.copy_text(uptime()));
        for error in &snapshot.errors {
            eprintln!("{error}");
        }
        eprintln!(
            "hardware rows={}, devices={}",
            snapshot.rows.len(),
            snapshot
                .rows
                .iter()
                .map(|row| row.devices.len())
                .sum::<usize>()
        );
        assert!(!snapshot.model.is_empty(), "local MI computer query failed");
        assert!(
            !snapshot.os.is_empty(),
            "local MI operating system query failed"
        );
        assert!(snapshot.rows.len() <= 9);
        for kind in [
            HardwareKind::Mainboard,
            HardwareKind::Processor,
            HardwareKind::Memory,
        ] {
            assert!(
                snapshot
                    .rows
                    .iter()
                    .any(|row| row.kind == kind && !row.devices.is_empty())
            );
        }
        assert!(snapshot.errors.is_empty(), "hardware summary query failed");
    }
}
