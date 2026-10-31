//! Afspeelwachtrij: volgorde, shuffle en herhalen. Puur logica, zonder afspeelengine.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    One,
    All,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueItem {
    pub location: String,
    pub title: Option<String>,
}

/// Kleine, deterministische RNG (xorshift64*), zodat shuffle testbaar is zonder externe crate.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
}

#[derive(Debug, Clone)]
pub struct Queue {
    items: Vec<QueueItem>,
    /// Afspeelvolgorde als indexen in `items`.
    order: Vec<usize>,
    /// Positie binnen `order`.
    cursor: Option<usize>,
    repeat: RepeatMode,
    shuffle: bool,
    rng: Rng,
}

impl Queue {
    pub fn new(seed: u64) -> Queue {
        Queue { items: vec![], order: vec![], cursor: None, repeat: RepeatMode::Off, shuffle: false, rng: Rng::new(seed) }
    }

    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }
    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Index (in `items`) van het huidige nummer.
    pub fn current_index(&self) -> Option<usize> {
        self.cursor.map(|c| self.order[c])
    }
    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index().map(|i| &self.items[i])
    }

    fn reshuffle_keeping(&mut self, keep: Option<usize>) {
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        if self.shuffle {
            // Fisher-Yates
            for i in (1..order.len()).rev() {
                let j = self.rng.below(i + 1);
                order.swap(i, j);
            }
            if let Some(k) = keep {
                // het huidige nummer blijft vooraan
                if let Some(p) = order.iter().position(|x| *x == k) {
                    order.swap(0, p);
                }
            }
        }
        self.cursor = keep.and_then(|k| order.iter().position(|x| *x == k));
        self.order = order;
    }

    pub fn set_shuffle(&mut self, on: bool) {
        if self.shuffle == on {
            return;
        }
        self.shuffle = on;
        let keep = self.current_index();
        self.reshuffle_keeping(keep);
    }

    pub fn set_repeat(&mut self, r: RepeatMode) {
        self.repeat = r;
    }

    /// Voegt toe aan het einde. Geeft de index van het eerste toegevoegde item.
    pub fn append(&mut self, new: impl IntoIterator<Item = QueueItem>) -> usize {
        let first = self.items.len();
        self.items.extend(new);
        for i in first..self.items.len() {
            // nieuwe items komen achteraan, ook bij shuffle (vooraf gemixte volgorde blijft staan)
            self.order.push(i);
        }
        if self.shuffle && self.items.len() > first {
            let keep = self.current_index();
            self.reshuffle_keeping(keep);
        }
        first
    }

    /// Vervangt de hele wachtrij en begint bij `start` (index in `items`).
    pub fn replace(&mut self, items: Vec<QueueItem>, start: usize) -> Result<()> {
        if start >= items.len() && !items.is_empty() {
            return Err(CoreError::Invalid("Startpositie ligt buiten de lijst".into()));
        }
        self.items = items;
        let keep = if self.items.is_empty() { None } else { Some(start) };
        self.reshuffle_keeping(keep);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.order.clear();
        self.cursor = None;
    }

    /// Springt naar een item (index in `items`).
    pub fn jump_to(&mut self, index: usize) -> Result<&QueueItem> {
        let pos = self.order.iter().position(|i| *i == index).ok_or_else(|| CoreError::NotFound(format!("item {index}")))?;
        self.cursor = Some(pos);
        Ok(&self.items[index])
    }

    pub fn remove(&mut self, index: usize) -> Result<()> {
        if index >= self.items.len() {
            return Err(CoreError::NotFound(format!("item {index}")));
        }
        let current = self.current_index();
        self.items.remove(index);
        let new_current = match current {
            Some(c) if c == index => None, // het spelende item is weg
            Some(c) if c > index => Some(c - 1),
            other => other,
        };
        // volgorde herbouwen: behoud de relatieve volgorde van de overige items
        self.order = self.order.iter().filter(|i| **i != index).map(|i| if *i > index { *i - 1 } else { *i }).collect();
        self.cursor = new_current.and_then(|c| self.order.iter().position(|x| *x == c));
        Ok(())
    }

    /// Verplaatst een item in de lijst (de weergavevolgorde); het huidige nummer blijft hetzelfde nummer.
    pub fn move_item(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.items.len() || to >= self.items.len() {
            return Err(CoreError::NotFound("positie".into()));
        }
        let current = self.current_index();
        let item = self.items.remove(from);
        self.items.insert(to, item);
        let remap = |i: usize| -> usize {
            if i == from {
                to
            } else if from < to && i > from && i <= to {
                i - 1
            } else if from > to && i >= to && i < from {
                i + 1
            } else {
                i
            }
        };
        self.order = self.order.iter().map(|i| remap(*i)).collect();
        self.cursor = current.map(remap).and_then(|c| self.order.iter().position(|x| *x == c));
        Ok(())
    }

    /// Welk item volgt na het huidige (bij natuurlijk einde)? Houdt rekening met herhalen.
    pub fn next_after_end(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }
        match (self.repeat, self.cursor) {
            (RepeatMode::One, Some(_)) => {}
            _ => return self.step(1, false),
        }
        self.current()
    }

    /// Handmatig volgende/vorige. Bij "Herhaal één" gaat dit gewoon naar het volgende item.
    pub fn go_next(&mut self) -> Option<&QueueItem> {
        self.step(1, true)
    }
    pub fn go_previous(&mut self) -> Option<&QueueItem> {
        self.step(-1, true)
    }

    fn step(&mut self, dir: i64, manual: bool) -> Option<&QueueItem> {
        let n = self.order.len() as i64;
        if n == 0 {
            return None;
        }
        let cur = match self.cursor {
            Some(c) => c as i64,
            None => {
                self.cursor = Some(if dir > 0 { 0 } else { (n - 1) as usize });
                return self.current();
            }
        };
        let target = cur + dir;
        if (0..n).contains(&target) {
            self.cursor = Some(target as usize);
        } else {
            // Buiten de lijst: alleen doorgaan bij "herhaal alles", of bij handmatig
            // bladeren met "herhaal één".
            let wrap = self.repeat == RepeatMode::All || (manual && self.repeat == RepeatMode::One);
            if !wrap {
                return None;
            }
            self.cursor = Some(target.rem_euclid(n) as usize);
            if self.shuffle && dir > 0 {
                // nieuwe ronde: nieuwe willekeurige volgorde
                self.reshuffle_keeping(None);
                self.cursor = Some(0);
            }
        }
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(n: usize) -> Vec<QueueItem> {
        (0..n).map(|i| QueueItem { location: format!("/m/{i}.mp3"), title: None }).collect()
    }
    fn q(n: usize) -> Queue {
        let mut q = Queue::new(42);
        q.replace(items(n), 0).unwrap();
        q
    }
    fn loc(q: &Queue) -> String {
        q.current().unwrap().location.clone()
    }

    #[test]
    fn plays_in_order_and_stops_at_the_end_without_repeat() {
        let mut q = q(3);
        assert_eq!(loc(&q), "/m/0.mp3");
        assert_eq!(q.next_after_end().unwrap().location, "/m/1.mp3");
        assert_eq!(q.next_after_end().unwrap().location, "/m/2.mp3");
        assert!(q.next_after_end().is_none());
        assert_eq!(loc(&q), "/m/2.mp3"); // blijft op het laatste staan
    }

    #[test]
    fn repeat_all_wraps_and_repeat_one_stays() {
        let mut a = q(2);
        a.set_repeat(RepeatMode::All);
        a.next_after_end();
        assert_eq!(a.next_after_end().unwrap().location, "/m/0.mp3");
        let mut b = q(3);
        b.set_repeat(RepeatMode::One);
        assert_eq!(b.next_after_end().unwrap().location, "/m/0.mp3");
        assert_eq!(b.next_after_end().unwrap().location, "/m/0.mp3");
        // handmatig volgende gaat bij "herhaal één" wel verder
        assert_eq!(b.go_next().unwrap().location, "/m/1.mp3");
    }

    #[test]
    fn manual_next_and_previous_respect_the_edges() {
        let mut q = q(3);
        assert!(q.go_previous().is_none()); // voor het eerste item, geen herhaling
        assert_eq!(q.go_next().unwrap().location, "/m/1.mp3");
        assert_eq!(q.go_previous().unwrap().location, "/m/0.mp3");
        q.jump_to(2).unwrap();
        assert!(q.go_next().is_none());
        q.set_repeat(RepeatMode::All);
        assert_eq!(q.go_next().unwrap().location, "/m/0.mp3");
        assert_eq!(q.go_previous().unwrap().location, "/m/2.mp3");
    }

    #[test]
    fn shuffle_plays_every_item_exactly_once_and_keeps_the_current_first() {
        let mut q = q(20);
        q.jump_to(7).unwrap();
        q.set_shuffle(true);
        assert_eq!(q.current_index(), Some(7));
        let mut seen = vec![q.current_index().unwrap()];
        while let Some(it) = q.next_after_end() {
            let idx: usize = it.location.trim_start_matches("/m/").trim_end_matches(".mp3").parse().unwrap();
            seen.push(idx);
        }
        assert_eq!(seen.len(), 20);
        let mut sorted = seen.clone();
        sorted.sort();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_ne!(seen, (0..20).collect::<Vec<_>>()); // echt door elkaar
    }

    #[test]
    fn turning_shuffle_off_keeps_the_current_item_and_restores_order() {
        let mut q = q(10);
        q.jump_to(4).unwrap();
        q.set_shuffle(true);
        q.go_next();
        let now = q.current_index().unwrap();
        q.set_shuffle(false);
        assert_eq!(q.current_index(), Some(now));
        let expected = if now < 9 { Some(format!("/m/{}.mp3", now + 1)) } else { None };
        assert_eq!(q.go_next().map(|i| i.location.clone()), expected);
    }

    #[test]
    fn append_remove_and_move_keep_the_current_item_correct() {
        let mut q = q(4);
        q.jump_to(2).unwrap();
        q.remove(0).unwrap(); // vóór het huidige: huidige schuift op
        assert_eq!(loc(&q), "/m/2.mp3");
        assert_eq!(q.current_index(), Some(1));
        q.remove(1).unwrap(); // het huidige item zelf
        assert!(q.current().is_none());
        assert_eq!(q.len(), 2);
        let first = q.append(items(2));
        assert_eq!(first, 2);
        assert_eq!(q.len(), 4);
        q.jump_to(0).unwrap();
        q.move_item(0, 3).unwrap();
        assert_eq!(loc(&q), "/m/1.mp3"); // oorspronkelijk item 0 is wat de cursor volgt
        assert_eq!(q.items()[3].location, "/m/1.mp3");
        assert!(q.remove(99).is_err() && q.move_item(0, 99).is_err() && q.jump_to(99).is_err());
    }

    #[test]
    fn empty_queue_is_safe_everywhere() {
        let mut q = Queue::new(1);
        assert!(q.go_next().is_none() && q.go_previous().is_none() && q.next_after_end().is_none() && q.current().is_none());
        q.set_shuffle(true);
        q.set_repeat(RepeatMode::All);
        assert!(q.next_after_end().is_none());
        assert!(q.replace(vec![], 0).is_ok());
        assert!(q.replace(items(2), 5).is_err());
    }

    #[test]
    fn duplicates_are_allowed() {
        let mut q = Queue::new(1);
        q.replace(vec![items(1)[0].clone(), items(1)[0].clone()], 0).unwrap();
        assert_eq!(q.len(), 2);
        q.go_next();
        assert_eq!(q.current_index(), Some(1));
    }

    #[test]
    fn rng_is_deterministic_and_covers_the_range() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(a.next_u64(), b.next_u64());
        let mut hit = [false; 5];
        for _ in 0..200 {
            hit[a.below(5)] = true;
        }
        assert!(hit.iter().all(|h| *h));
        assert_eq!(Rng::new(0).below(0), 0);
    }
}
