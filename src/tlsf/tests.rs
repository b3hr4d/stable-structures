use super::*;
use proptest::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use tiny_rng::{Rand, Rng};

fn make_memory() -> Rc<RefCell<Vec<u8>>> {
    Rc::new(RefCell::new(Vec::new()))
}

#[test]
fn deallocate_everything() {
    proptest!(|(
        data in proptest::collection::vec(
            proptest::collection::vec(0..u8::MAX, 3usize..100usize), 3..20
        ),
    )| {
        let mut rng = Rng::from_seed(0);
        let mem = make_memory();
        let mut tlsf = TlsfAllocator::new(mem);
        let mut addresses: Vec<(Address, Vec<u8>)> = vec![];

        for d in data.into_iter() {
            let address = tlsf.allocate(d.len() as u32);

            // Write the data into memory.
            tlsf.memory.write(address.get(), &d);

            addresses.push((address, d));
        }

        // Shuffle addresses to deallocate them in random order.
        rng.shuffle(&mut addresses);
        for (address, data) in addresses {
            // Read data from memory and verify its there.
            let mut v = vec![0; data.len()];
            tlsf.memory.read(address.get(), &mut v);
            assert_eq!(v, data);

            tlsf.deallocate(address);
        }

        prop_assert_eq!(
            Block::load(DATA_OFFSET, &tlsf.memory),
            Block {
                address: DATA_OFFSET,
                allocated: false,
                size: MEMORY_POOL_SIZE,
                prev_free: Address::NULL,
                next_free: Address::NULL,
                prev_physical: Address::NULL,
            }
        );

        prop_assert_eq!(
            tlsf.free_lists,
            TlsfAllocator::new(make_memory()).free_lists
        );
    });
}

#[test]
fn v2_deallocate_everything() {
    let data = vec![
        vec![0, 0, 0],
        vec![0, 0, 0],
   //     vec![0, 0, 0],
  //      vec![0, 0, 0],
    //    vec![0, 0, 0],
    ];
    let mem = make_memory();
    let mut tlsf = TlsfAllocator::new(mem);
    let mut addresses: Vec<(Address, Vec<u8>)> = vec![];

    for d in data.into_iter() {
        let address = tlsf.allocate(d.len() as u32);

        // Write the data into memory.
        tlsf.memory.write(address.get(), &d);

        addresses.push((address, d));
    }

    // Shuffle addresses to deallocate them in random order.
    //rng.shuffle(&mut addresses);
    for (address, data) in addresses {
        // Read data from memory and verify its there.
        let mut v = vec![0; data.len()];
        tlsf.memory.read(address.get(), &mut v);
        assert_eq!(v, data);

        tlsf.deallocate(address);
    }

    assert_eq!(
        Block::load(DATA_OFFSET, &tlsf.memory),
        Block {
            address: DATA_OFFSET,
            allocated: false,
            size: MEMORY_POOL_SIZE,
            prev_free: Address::NULL,
            next_free: Address::NULL,
            prev_physical: Address::NULL,
        }
    );

    assert_eq!(
        tlsf.free_lists,
        TlsfAllocator::new(make_memory()).free_lists
    );
}

#[test]
fn multiple_allocations_no_deallocations() {
    proptest!(|(
        data in proptest::collection::vec(
            proptest::collection::vec(0..u8::MAX, 3usize..100usize), 1..100
        ),
    )| {
        let mem = make_memory();
        let mut tlsf = TlsfAllocator::new(mem);
        let mut addresses: Vec<(Address, Vec<u8>)> = vec![];

        let mut offset = Bytes::new(0);
        for d in data.into_iter() {
            let address = tlsf.allocate(d.len() as u32);

            // Asserts that the free lists have been updated accordingly.
            offset += Bytes::from(Block::header_size()) + Bytes::from(d.len() as u64);
            prop_assert_eq!(
                tlsf.free_lists[FIRST_LEVEL_INDEX_SIZE - 1][SECOND_LEVEL_INDEX_SIZE - 1],
                DATA_OFFSET + offset
            );

            // Write the data into memory.
            tlsf.memory.write(address.get(), &d);
            addresses.push((address, d));
        }

        // Read all the data from memory.
        let mut v = Vec::new();
        for (address, data) in addresses {
            v.resize(data.len(), 0);
            // Read data from memory and verify its there.
            tlsf.memory.read(address.get(), &mut v);
            prop_assert_eq!(&v, &data);
        }
    });
}
