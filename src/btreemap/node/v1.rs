use super::*;

use crate::types::NULL;
use crate::write_u64;

impl<K: Storable + Ord + Clone> Node<K> {
    /// Loads a node from memory at the given address.
    pub(super) fn load_v1<M: Memory>(
        address: Address,
        memory: &M,
        max_key_size: u32,
        max_value_size: u32,
    ) -> Self {
        // Load the metadata.
        let mut offset = META_DATA_OFFSET;
        let mut buf = vec![0];
        memory.read((address + offset).get(), &mut buf);
        let node_type = match buf[0] {
            LEAF_NODE_TYPE => NodeType::Leaf,
            INTERNAL_NODE_TYPE => NodeType::Internal,
            other => unreachable!("Unknown node type {}", other),
        };

        offset += Bytes::new(1);

        // TODO: add read u16?
        buf.resize(2, 0);
        memory.read((address + offset).get(), &mut buf);
        let num_entries = u16::from_le_bytes(buf.try_into().unwrap()) as usize;

        // Load the entries.
        let mut keys = Vec::with_capacity(num_entries);
        let mut encoded_values = Vec::with_capacity(num_entries);
        let mut offset = ENTRIES_OFFSET_V1;
        let mut buf = Vec::with_capacity(max_key_size.max(max_value_size) as usize);
        for _ in 0..num_entries {
            // Read the key's size.
            let key_size = read_u32(memory, address + offset);
            offset += U32_SIZE;

            // Read the key.
            buf.resize(key_size as usize, 0);
            memory.read((address + offset).get(), &mut buf);
            offset += Bytes::from(max_key_size);
            let key = K::from_bytes(Cow::Borrowed(&buf));
            keys.push(key);

            // Values are loaded lazily. Store a reference and skip loading it.
            encoded_values.push(Value::ByRef(offset));
            offset += U32_SIZE + Bytes::from(max_value_size);
        }

        // Load children if this is an internal node.
        let mut children = vec![];
        if node_type == NodeType::Internal {
            // The number of children is equal to the number of entries + 1.
            for _ in 0..num_entries + 1 {
                let child = Address::from(read_u64(memory, address + offset));
                offset += Address::size();
                children.push(child);
            }

            assert_eq!(children.len(), keys.len() + 1);
        }

        Self {
            address,
            keys,
            encoded_values: RefCell::new(encoded_values),
            children,
            node_type,
            version: Version::V1 {
                max_key_size,
                max_value_size,
            },
            overflow: None,
        }
    }

    pub(super) fn save_v1<M: Memory>(&self, max_key_size: u32, max_value_size: u32, memory: &M) {
        memory.write(self.address.get(), MAGIC);
        memory.write((self.address + Bytes::new(3)).get(), &[LAYOUT_VERSION]);
        let header = NodeHeader {
            node_type: match self.node_type {
                NodeType::Leaf => LEAF_NODE_TYPE,
                NodeType::Internal => INTERNAL_NODE_TYPE,
            },
            num_entries: self.keys.len() as u16,
        };

        // the u32 offset is to not overwrite the magic and version
        write_struct(&header, self.address + U32_SIZE, memory);

        let mut offset = ENTRIES_OFFSET_V1;

        // Load all the values. This is necessary so that we don't overwrite referenced
        // values when writing the entries to the node.
        for i in 0..self.keys.len() {
            self.value(i, memory);
        }

        // Write the entries.
        for (idx, key) in self.keys.iter().enumerate() {
            // Write the size of the key.
            let key_bytes = key.to_bytes();
            write_u32(memory, self.address + offset, key_bytes.len() as u32);
            offset += U32_SIZE;

            // Write the key.
            write(memory, (self.address + offset).get(), key_bytes.borrow());
            offset += Bytes::from(max_key_size);

            // Write the size of the value.
            let value = self.value(idx, memory);
            write_u32(memory, self.address + offset, value.len() as u32);
            offset += U32_SIZE;

            // Write the value.
            write(memory, (self.address + offset).get(), &value);
            offset += Bytes::from(max_value_size);
        }

        // Write the children
        for child in self.children.iter() {
            write(
                memory,
                (self.address + offset).get(),
                &child.get().to_le_bytes(),
            );
            offset += Address::size();
        }
    }
}
