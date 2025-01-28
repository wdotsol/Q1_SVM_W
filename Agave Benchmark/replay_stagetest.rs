use core::vote_similator::{self, VoteSimulator};
use core:ReplayStage::ReplayBlockstoreComponents;


fn test_child_slots_of_same_parent() {
    let ReplayBlockstoreComponents {
        blockstore,
        validator_node_to_vote_keys,
        vote_simulator,
        leader_schedule_cache,
        rpc_subscriptions,
        ..
    } = replay_blockstore_components(None, 1, None::<GenerateVotes>);

    let VoteSimulator {
        mut progress,
        bank_forks,
        ..
    } = vote_simulator;

    // Insert a non-root bank so that the propagation logic will update this
    // bank
    let bank1 = Bank::new_from_parent(
        bank_forks.read().unwrap().get(0).unwrap(),
        &leader_schedule_cache.slot_leader_at(1, None).unwrap(),
        1,
    );
    progress.insert(
        1,
        ForkProgress::new_from_bank(
            &bank1,
            bank1.collector_id(),
            validator_node_to_vote_keys
                .get(bank1.collector_id())
                .unwrap(),
            Some(0),
            0,
            0,
        ),
    );
    assert!(progress.get_propagated_stats(1).unwrap().is_leader_slot);
    bank1.freeze();
    bank_forks.write().unwrap().insert(bank1);

    // Insert shreds for slot NUM_CONSECUTIVE_LEADER_SLOTS,
    // chaining to slot 1
    let (shreds, _) = make_slot_entries(
        NUM_CONSECUTIVE_LEADER_SLOTS, // slot
        1,                            // parent_slot
        8,                            // num_entries
        true,                         // merkle_variant
    );
    blockstore.insert_shreds(shreds, None, false).unwrap();
    assert!(bank_forks
        .read()
        .unwrap()
        .get(NUM_CONSECUTIVE_LEADER_SLOTS)
        .is_none());
    let mut replay_timing = ReplayLoopTiming::default();
    ReplayStage::generate_new_bank_forks(
        &blockstore,
        &bank_forks,
        &leader_schedule_cache,
        &rpc_subscriptions,
        &None,
        &mut progress,
        &mut replay_timing,
    );
    assert!(bank_forks
        .read()
        .unwrap()
        .get(NUM_CONSECUTIVE_LEADER_SLOTS)
        .is_some());

    // Insert shreds for slot 2 * NUM_CONSECUTIVE_LEADER_SLOTS,
    // chaining to slot 1
    let (shreds, _) = make_slot_entries(
        2 * NUM_CONSECUTIVE_LEADER_SLOTS,
        1,
        8,
        true, // merkle_variant
    );
    blockstore.insert_shreds(shreds, None, false).unwrap();
    assert!(bank_forks
        .read()
        .unwrap()
        .get(2 * NUM_CONSECUTIVE_LEADER_SLOTS)
        .is_none());
    ReplayStage::generate_new_bank_forks(
        &blockstore,
        &bank_forks,
        &leader_schedule_cache,
        &rpc_subscriptions,
        &None,
        &mut progress,
        &mut replay_timing,
    );
    assert!(bank_forks
        .read()
        .unwrap()
        .get(NUM_CONSECUTIVE_LEADER_SLOTS)
        .is_some());
    assert!(bank_forks
        .read()
        .unwrap()
        .get(2 * NUM_CONSECUTIVE_LEADER_SLOTS)
        .is_some());

    // // There are 20 equally staked accounts, of which 3 have built
    // banks above or at bank 1. Because 3/20 < SUPERMINORITY_THRESHOLD,
    // we should see 3 validators in bank 1's propagated_validator set.
    let expected_leader_slots = vec![
        1,
        NUM_CONSECUTIVE_LEADER_SLOTS,
        2 * NUM_CONSECUTIVE_LEADER_SLOTS,
    ];
    for slot in expected_leader_slots {
        let leader = leader_schedule_cache.slot_leader_at(slot, None).unwrap();
        let vote_key = validator_node_to_vote_keys.get(&leader).unwrap();
        assert!(progress
            .get_propagated_stats(1)
            .unwrap()
            .propagated_validators
            .contains(vote_key));
    }
}