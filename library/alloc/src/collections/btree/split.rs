use super::map::MIN_LEN;
use super::node::{ForceResult::*, Root};
use super::search::SearchResult::*;
use core::borrow::Borrow;

impl<K, V> Root<K, V> {
    /// Calculates the length of both trees that result from splitting up
    /// a given number of distinct key-value pairs.
    pub fn calc_split_length(
        total_num: usize,
        root_a: &Root<K, V>,
        root_b: &Root<K, V>,
    ) -> (usize, usize) {
        let (length_a, length_b);
        if root_a.height() < root_b.height() {
            length_a = root_a.reborrow().calc_length();
            length_b = total_num - length_a;
            debug_assert_eq!(length_b, root_b.reborrow().calc_length());
        } else {
            length_b = root_b.reborrow().calc_length();
            length_a = total_num - length_b;
            debug_assert_eq!(length_a, root_a.reborrow().calc_length());
        }
        (length_a, length_b)
    }

    /// Split off a tree with key-value pairs at and after the given key.
    /// The result is meaningful only if the tree is ordered by key,
    /// and if the ordering of `Q` corresponds to that of `K`.
    /// If `self` respects all `BTreeMap` tree invariants, then both
    /// `self` and the returned tree will respect those invariants.
    pub fn split_off<Q: ?Sized + Ord>(&mut self, key: &Q) -> Self
    where
        K: Borrow<Q>,
    {
        let left_root = self;
        let mut right_root = Root::new_pillar(left_root.height());
        let mut left_node = left_root.borrow_mut();
        let mut right_node = right_root.borrow_mut();

        loop {
            let mut split_edge = match left_node.search_node(key) {
                // key is going to the right tree
                Found(kv) => kv.left_edge(),
                GoDown(edge) => edge,
            };

            split_edge.move_suffix(&mut right_node);

            match (split_edge.force(), right_node.force()) {
                (Internal(edge), Internal(node)) => {
                    left_node = edge.descend();
                    right_node = node.first_edge().descend();
                }
                (Leaf(_), Leaf(_)) => break,
                _ => unreachable!(),
            }
        }

        left_root.fix_right_border();
        right_root.fix_left_border();
        right_root
    }

    /// Creates a tree consisting of empty nodes.
    fn new_pillar(height: usize) -> Self {
        let mut root = Root::new();
        for _ in 0..height {
            root.push_internal_level();
        }
        root
    }

    /// Removes empty levels on the top, but keeps an empty leaf if the entire tree is empty.
    fn fix_top(&mut self) {
        while self.height() > 0 && self.len() == 0 {
            self.pop_internal_level();
        }
    }

    /// Stock up or merge away any underfull nodes on the right border of the
    /// tree. The other nodes, those that are not the root nor a rightmost edge,
    /// must already have at least MIN_LEN elements.
    fn fix_right_border(&mut self) {
        self.fix_top();

        {
            let mut cur_node = self.borrow_mut();

            while let Internal(node) = cur_node.force() {
                let mut last_kv = node.last_kv().consider_for_balancing();

                if last_kv.can_merge() {
                    cur_node = last_kv.merge_tracking_child();
                } else {
                    let right_len = last_kv.right_child_len();
                    // `MIN_LEN + 1` to avoid readjust if merge happens on the next level.
                    if right_len < MIN_LEN + 1 {
                        last_kv.bulk_steal_left(MIN_LEN + 1 - right_len);
                    }
                    cur_node = last_kv.into_right_child();
                }
                debug_assert!(cur_node.len() > MIN_LEN);
            }
        }

        self.fix_top();
    }

    /// The symmetric clone of `fix_right_border`.
    fn fix_left_border(&mut self) {
        self.fix_top();

        {
            let mut cur_node = self.borrow_mut();

            while let Internal(node) = cur_node.force() {
                let mut first_kv = node.first_kv().consider_for_balancing();

                if first_kv.can_merge() {
                    cur_node = first_kv.merge_tracking_child();
                } else {
                    let left_len = first_kv.left_child_len();
                    // `MIN_LEN + 1` to avoid readjust if merge happens on the next level.
                    if left_len < MIN_LEN + 1 {
                        first_kv.bulk_steal_right(MIN_LEN + 1 - left_len);
                    }
                    cur_node = first_kv.into_left_child();
                }
                debug_assert!(cur_node.len() > MIN_LEN);
            }
        }

        self.fix_top();
    }
}
