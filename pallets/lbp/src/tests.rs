use super::*;
pub use crate::mock::{
	run_to_block, Currency, Event as TestEvent, ExtBuilder, LBPPallet, Origin, System, Test, ACA, ALICE, BOB, DOT, ETH,
	HDX,
};
use crate::mock::{ACA_DOT_POOL_ID, HDX_DOT_POOL_ID, INITIAL_BALANCE, POOL_DEPOSIT};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext = ExtBuilder::default().build();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub use primitives::{asset::AssetPair, traits::AMMTransfer, MAX_IN_RATIO, MAX_OUT_RATIO};

pub fn predefined_test_ext() -> sp_io::TestExternalities {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: ACA,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: DOT,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(10u64, 20u64),
			WeightCurveType::Linear,
			true,
		));

		assert_ok!(LBPPallet::unpause_pool(Origin::signed(ALICE), ACA_DOT_POOL_ID));

		assert_eq!(
			<PoolData<Test>>::get(ACA_DOT_POOL_ID),
			Pool {
				start: 10u64,
				end: 20u64,
				initial_weights: ((ACA, 20), (DOT, 80)),
				final_weights: ((ACA, 90), (DOT, 10)),
				last_weight_update: 0_u64,
				last_weights: ((ACA, 20), (DOT, 80)),
				curve: WeightCurveType::Linear,
				pausable: true,
				paused: false,
			}
		);
	});
	ext
}

fn last_events(n: usize) -> Vec<TestEvent> {
	frame_system::Pallet::<Test>::events()
		.into_iter()
		.rev()
		.take(n)
		.rev()
		.map(|e| e.event)
		.collect()
}

fn expect_events(e: Vec<TestEvent>) {
	assert_eq!(last_events(e.len()), e);
}

#[test]
fn weight_update_should_work() {
	new_test_ext().execute_with(|| {
		let asset_a = LBPAssetInfo {
			id: HDX,
			amount: 1,
			initial_weight: 20,
			final_weight: 80,
		};
		let asset_b = LBPAssetInfo {
			id: DOT,
			amount: 2,
			initial_weight: 80,
			final_weight: 20,
		};
		let asset_c = LBPAssetInfo {
			id: ACA,
			amount: 2,
			initial_weight: 80,
			final_weight: 20,
		};
		let duration = (10u64, 19u64);

		let mut linear_pool = Pool::new(asset_a, asset_b, duration, WeightCurveType::Linear, false);
		let mut constant_pool = Pool::new(asset_a, asset_c, duration, WeightCurveType::Constant, false);

		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			asset_a,
			asset_b,
			duration,
			WeightCurveType::Linear,
			false
		));
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			asset_a,
			asset_c,
			duration,
			WeightCurveType::Constant,
			false
		));

		System::set_block_number(13);

		assert_ok!(LBPPallet::update_weights(&HDX_DOT_POOL_ID, &mut linear_pool));
		assert_ok!(LBPPallet::update_weights(&ACA_DOT_POOL_ID, &mut constant_pool));

		let mut linear_pool = LBPPallet::pool_data(HDX_DOT_POOL_ID);
		let mut constant_pool = LBPPallet::pool_data(ACA_DOT_POOL_ID);

		assert_eq!(linear_pool.last_weight_update, 13);
		assert_eq!(constant_pool.last_weight_update, 13);

		assert_eq!(linear_pool.last_weights, ((HDX, 40u128), (DOT, 60u128)));
		assert_eq!(constant_pool.last_weights, ((HDX, 20u128), (ACA, 80u128)));

		// call update again in the same block, data should be the same
		assert_ok!(LBPPallet::update_weights(&HDX_DOT_POOL_ID, &mut linear_pool));
		assert_ok!(LBPPallet::update_weights(&ACA_DOT_POOL_ID, &mut constant_pool));

		let linear_pool = LBPPallet::pool_data(HDX_DOT_POOL_ID);
		let constant_pool = LBPPallet::pool_data(ACA_DOT_POOL_ID);

		assert_eq!(linear_pool.last_weight_update, 13);
		assert_eq!(constant_pool.last_weight_update, 13);

		assert_eq!(linear_pool.last_weights, ((HDX, 40u128), (DOT, 60u128)));
		assert_eq!(constant_pool.last_weights, ((HDX, 20u128), (ACA, 80u128)));
	});
}

#[test]
fn validate_pool_data_should_work() {
	new_test_ext().execute_with(|| {
		let pool_data = Pool {
			start: 10u64,
			end: 20u64,
			initial_weights: ((ACA, 20), (DOT, 80)),
			final_weights: ((ACA, 90), (DOT, 10)),
			last_weight_update: 0u64,
			last_weights: ((ACA, 20), (DOT, 80)),
			curve: WeightCurveType::Linear,
			pausable: true,
			paused: false,
		};
		assert_ok!(LBPPallet::validate_pool_data(&pool_data));

		// null interval
		let pool_data = Pool {
			start: 0u64,
			end: 0u64,
			initial_weights: ((ACA, 20), (DOT, 80)),
			final_weights: ((ACA, 90), (DOT, 10)),
			last_weight_update: 0u64,
			last_weights: ((ACA, 20), (DOT, 80)),
			curve: WeightCurveType::Linear,
			pausable: true,
			paused: false,
		};
		assert_ok!(LBPPallet::validate_pool_data(&pool_data));

		let pool_data = Pool {
			start: 10u64,
			end: 2u64,
			initial_weights: ((ACA, 20), (DOT, 80)),
			final_weights: ((ACA, 90), (DOT, 10)),
			last_weight_update: 0u64,
			last_weights: ((ACA, 20), (DOT, 80)),
			curve: WeightCurveType::Linear,
			pausable: true,
			paused: false,
		};
		assert_noop!(
			LBPPallet::validate_pool_data(&pool_data),
			Error::<Test>::InvalidBlockNumber
		);

		let pool_data = Pool {
			start: 10u64,
			end: 11u64 + u32::MAX as u64,
			initial_weights: ((ACA, 20), (DOT, 80)),
			final_weights: ((ACA, 90), (DOT, 10)),
			last_weight_update: 0u64,
			last_weights: ((ACA, 20), (DOT, 80)),
			curve: WeightCurveType::Linear,
			pausable: true,
			paused: false,
		};
		assert_noop!(
			LBPPallet::validate_pool_data(&pool_data),
			Error::<Test>::MaxSaleDurationExceeded
		);
	});
}

#[test]
fn create_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: ACA,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: DOT,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(10u64, 20u64),
			WeightCurveType::Linear,
			true,
		));

		assert_eq!(Currency::free_balance(ACA, &ACA_DOT_POOL_ID), 1_000_000_000);
		assert_eq!(Currency::free_balance(DOT, &ACA_DOT_POOL_ID), 2_000_000_000);
		assert_eq!(
			Currency::free_balance(ACA, &ALICE),
			INITIAL_BALANCE.saturating_sub(1_000_000_000)
		);
		assert_eq!(
			Currency::free_balance(DOT, &ALICE),
			INITIAL_BALANCE.saturating_sub(2_000_000_000)
		);
		assert_eq!(Currency::reserved_balance(HDX, &ALICE), POOL_DEPOSIT);
		assert_eq!(
			Currency::free_balance(HDX, &ALICE),
			INITIAL_BALANCE.saturating_sub(POOL_DEPOSIT)
		);

		assert_eq!(LBPPallet::pool_owner(&ACA_DOT_POOL_ID), ALICE);
		assert_eq!(LBPPallet::pool_deposit(&ACA_DOT_POOL_ID), POOL_DEPOSIT);
		assert_eq!(LBPPallet::pool_assets(&ACA_DOT_POOL_ID), (ACA, DOT));
		assert_eq!(LBPPallet::pool_balances(&ACA_DOT_POOL_ID), (1_000_000_000, 2_000_000_000));

		let pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(pool_data.start, 10u64);
		assert_eq!(pool_data.end, 20u64);
		assert_eq!(pool_data.initial_weights, ((ACA, 20), (DOT, 80)));
		assert_eq!(pool_data.final_weights, ((ACA, 90), (DOT, 10)));
		assert_eq!(pool_data.curve, WeightCurveType::Linear);
		assert_eq!(pool_data.pausable, true);
		// verify that `last_weight_update`, `last_weights` and `paused` fields are correctly initialized
		assert_eq!(pool_data.last_weight_update, 0);
		assert_eq!(pool_data.last_weights, ((ACA, 20), (DOT, 80)));
		assert_eq!(pool_data.paused, true);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into()
		]);
	});
}

#[test]
fn create_pool_from_basic_origin_should_not_work() {
	new_test_ext().execute_with(|| {
		// only CreatePoolOrigin is allowed to create new pools
		assert_noop!(
			LBPPallet::create_pool(
				Origin::signed(ALICE),
				ALICE,
				LBPAssetInfo {
					id: HDX,
					amount: 1_000_000_000,
					initial_weight: 20,
					final_weight: 90,
				},
				LBPAssetInfo {
					id: DOT,
					amount: 2_000_000_000,
					initial_weight: 80,
					final_weight: 10,
				},
				(10u64, 20u64),
				WeightCurveType::Linear,
				true,
			),
			BadOrigin
		);
	});
}

#[test]
fn create_same_pool_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: ACA,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: DOT,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(10u64, 20u64),
			WeightCurveType::Linear,
			true,
		));

		assert_noop!(
			LBPPallet::create_pool(
				Origin::root(),
				ALICE,
				LBPAssetInfo {
					id: ACA,
					amount: 10_000_000_000,
					initial_weight: 30,
					final_weight: 70,
				},
				LBPAssetInfo {
					id: DOT,
					amount: 20_000_000_000,
					initial_weight: 70,
					final_weight: 30,
				},
				(100u64, 200u64),
				WeightCurveType::Linear,
				true,
			),
			Error::<Test>::PoolAlreadyExists
		);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into()
		]);
	});
}

#[test]
fn create_pool_with_same_assets_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::create_pool(
				Origin::root(),
				ALICE,
				LBPAssetInfo {
					id: ACA,
					amount: 1_000_000_000,
					initial_weight: 20,
					final_weight: 90,
				},
				LBPAssetInfo {
					id: ACA,
					amount: 2_000_000_000,
					initial_weight: 80,
					final_weight: 10,
				},
				(20u64, 10u64),
				WeightCurveType::Linear,
				true,
			),
			Error::<Test>::CannotCreatePoolWithSameAssets
		);
	});
}

#[test]
fn create_pool_invalid_data_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::create_pool(
				Origin::root(),
				ALICE,
				LBPAssetInfo {
					id: ACA,
					amount: 1_000_000_000,
					initial_weight: 20,
					final_weight: 90,
				},
				LBPAssetInfo {
					id: DOT,
					amount: 2_000_000_000,
					initial_weight: 80,
					final_weight: 10,
				},
				(20u64, 10u64), // reversed interval, the end precedes the beginning
				WeightCurveType::Linear,
				true,
			),
			Error::<Test>::InvalidBlockNumber
		);
	});
}

#[test]
fn update_pool_data_should_work() {
	predefined_test_ext().execute_with(|| {
		// update all parameters
		assert_ok!(LBPPallet::update_pool_data(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			Some(15),
			Some(18),
			Some(((ACA, 10), (DOT, 90))),
			Some(((ACA, 80), (DOT, 20))),
		));

		// verify changes
		let updated_pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(updated_pool_data.start, 15);
		assert_eq!(updated_pool_data.end, 18);
		assert_eq!(updated_pool_data.initial_weights, ((ACA, 10), (DOT, 90)));
		assert_eq!(updated_pool_data.final_weights, ((ACA, 80), (DOT, 20)));

		// update only one parameter
		assert_ok!(LBPPallet::update_pool_data(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			None,
			Some(30),
			None,
			None,
		));

		// verify changes
		let updated_pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(updated_pool_data.start, 15);
		assert_eq!(updated_pool_data.end, 30);
		assert_eq!(updated_pool_data.initial_weights, ((ACA, 10), (DOT, 90)));
		assert_eq!(updated_pool_data.final_weights, ((ACA, 80), (DOT, 20)));

		// update only one parameter
		assert_ok!(LBPPallet::update_pool_data(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			None,
			None,
			Some(((ACA, 10), (DOT, 70))),
			None,
		));

		// verify changes
		let updated_pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(updated_pool_data.start, 15);
		assert_eq!(updated_pool_data.end, 30);
		assert_eq!(updated_pool_data.initial_weights, ((ACA, 10), (DOT, 70)));
		assert_eq!(updated_pool_data.final_weights, ((ACA, 80), (DOT, 20)));

		// mix
		assert_ok!(LBPPallet::update_pool_data(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			None,
			Some(18),
			Some(((ACA, 10), (DOT, 90))),
			None,
		));

		// verify changes
		let updated_pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(updated_pool_data.start, 15);
		assert_eq!(updated_pool_data.end, 18);
		assert_eq!(updated_pool_data.initial_weights, ((ACA, 10), (DOT, 90)));
		assert_eq!(updated_pool_data.final_weights, ((ACA, 80), (DOT, 20)));

		expect_events(vec![
			Event::PoolUpdated(ALICE, ACA_DOT_POOL_ID).into(),
			Event::PoolUpdated(ALICE, ACA_DOT_POOL_ID).into(),
			Event::PoolUpdated(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn update_pool_data_without_changes_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(LBPPallet::update_pool_data(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			None,
			None,
			None,
			None,
		),
		Error::<Test>::NothingToUpdate);
	});
}

#[test]
fn update_pool_data_by_non_owner_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(LBPPallet::update_pool_data(
			Origin::signed(BOB),
			ACA_DOT_POOL_ID,
			Some(15),
			None,
			Some(((ACA, 10), (DOT, 90))),
			None,
		),
		Error::<Test>::NotOwner);
	});
}

#[test]
fn update_pool_data_for_running_lbp_should_not_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(16);

		// update starting block and final weights
		assert_noop!(
			LBPPallet::update_pool_data(
				Origin::signed(ALICE),
				ACA_DOT_POOL_ID,
				Some(15),
				None,
				Some(((1, 10), (2, 90))),
				None,
			),
			Error::<Test>::SaleStarted
		);

		let updated_pool_data = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(updated_pool_data.start, 10);
		assert_eq!(updated_pool_data.end, 20);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn pause_pool_should_work() {
	predefined_test_ext().execute_with(|| {
		assert_ok!(LBPPallet::pause_pool(Origin::signed(ALICE), ACA_DOT_POOL_ID));

		let paused_pool = LBPPallet::pool_data(ACA_DOT_POOL_ID);
		assert_eq!(
			paused_pool,
			Pool {
				start: 10u64,
				end: 20u64,
				initial_weights: ((ACA, 20), (DOT, 80)),
				final_weights: ((ACA, 90), (DOT, 10)),
				last_weight_update: 0u64,
				last_weights: ((ACA, 20), (DOT, 80)),
				curve: WeightCurveType::Linear,
				pausable: true,
				paused: true
			}
		);

		expect_events(vec![Event::Paused(ALICE, ACA_DOT_POOL_ID).into()]);
	});
}

#[test]
fn pause_non_existing_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let non_existing_id = 25486;
		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(ALICE), non_existing_id),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn pause_pool_by_non_owner_should_not_work() {
	predefined_test_ext().execute_with(|| {
		//user is not pool owner
		let not_owner = BOB;
		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(not_owner), ACA_DOT_POOL_ID),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn pause_non_puasable_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		//pool is not puasable
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			BOB,
			LBPAssetInfo {
				id: ACA,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 40,
			},
			LBPAssetInfo {
				id: ETH,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 60,
			},
			(200u64, 400u64),
			WeightCurveType::Linear,
			false,
		));

		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(BOB), 2_004_000),
			Error::<Test>::PoolIsNotPausable
		);
	});
}

#[test]
fn pause_paused_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		//pool is already paused
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			BOB,
			LBPAssetInfo {
				id: DOT,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 40,
			},
			LBPAssetInfo {
				id: ETH,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 60,
			},
			(200u64, 400u64),
			WeightCurveType::Linear,
			true,
		));

		// pool is already paused  - pool is created as puased by default
		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(BOB), 3_004_000),
			Error::<Test>::CannotPausePausedPool
		);
	});
}

#[test]
fn pause_non_running_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		//pool is ended or ending in current block
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: DOT,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 40,
			},
			LBPAssetInfo {
				id: HDX,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 60,
			},
			(200u64, 400u64),
			WeightCurveType::Linear,
			true,
		));

		//pool is created as puased by default
		assert_ok!(LBPPallet::unpause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID));

		run_to_block(400);
		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID),
			Error::<Test>::CannotPauseEndedPool
		);

		run_to_block(500);
		assert_noop!(
			LBPPallet::pause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID),
			Error::<Test>::CannotPauseEndedPool
		);
	});
}

#[test]
fn unpause_pool_should_work() {
	predefined_test_ext().execute_with(|| {
		//pool is created as puased by default
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: DOT,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 40,
			},
			LBPAssetInfo {
				id: HDX,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 60,
			},
			(200u64, 400u64),
			WeightCurveType::Linear,
			true,
		));

		assert_ok!(LBPPallet::unpause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID,));

		let unpaused_pool = LBPPallet::pool_data(HDX_DOT_POOL_ID);
		assert_eq!(
			unpaused_pool,
			Pool {
				start: 200_u64,
				end: 400_u64,
				initial_weights: ((DOT, 20), (HDX, 80)),
				final_weights: ((DOT, 40), (HDX, 60)),
				last_weight_update: 0u64,
				last_weights: ((DOT, 20), (HDX, 80)),
				curve: WeightCurveType::Linear,
				pausable: true,
				paused: false
			}
		);

		expect_events(vec![
			Event::PoolCreated(ALICE, HDX_DOT_POOL_ID, DOT, HDX, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, HDX_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn unpause_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		//user is not pool owner
		let not_owner = BOB;
		assert_noop!(
			LBPPallet::unpause_pool(Origin::signed(not_owner), ACA_DOT_POOL_ID),
			Error::<Test>::NotOwner
		);

		//pool is not found
		assert_noop!(
			LBPPallet::unpause_pool(Origin::signed(ALICE), 24568),
			Error::<Test>::PoolNotFound
		);

		//predefined_test_ext pool is unpaused
		assert_noop!(
			LBPPallet::unpause_pool(Origin::signed(ALICE), ACA_DOT_POOL_ID),
			Error::<Test>::PoolIsNotPaused
		);

		//pool is ended or ending in current block - pool is puased by default
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: DOT,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 40,
			},
			LBPAssetInfo {
				id: HDX,
				amount: 2_000_000_000,
				initial_weight: 80,
				final_weight: 60,
			},
			(200u64, 400u64),
			WeightCurveType::Linear,
			true,
		));

		run_to_block(400);
		assert_noop!(
			LBPPallet::unpause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID),
			Error::<Test>::CannotUnpauseEndedPool
		);

		run_to_block(500);
		assert_noop!(
			LBPPallet::unpause_pool(Origin::signed(ALICE), HDX_DOT_POOL_ID),
			Error::<Test>::CannotUnpauseEndedPool
		);
	});
}

#[test]
fn add_liquidity_should_work() {
	predefined_test_ext().execute_with(|| {
		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		let added_a = 10_000_000_000;
		let added_b = 20_000_000_000;

		assert_ok!(LBPPallet::add_liquidity(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			added_a,
			added_b,
		));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before.saturating_add(added_a));
		assert_eq!(balance_b_after, balance_b_before.saturating_add(added_b));

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before.saturating_sub(added_a));
		assert_eq!(user_balance_b_after, user_balance_b_before.saturating_sub(added_b));

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
			Event::LiquidityAdded(ACA_DOT_POOL_ID, ACA, DOT, added_a, added_b).into(),
		]);

		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_ok!(LBPPallet::add_liquidity(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			added_a,
			0,
		));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_eq!(balance_a_after, balance_a_before.saturating_add(added_a));
		assert_eq!(balance_b_after, balance_b_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
			Event::LiquidityAdded(ACA_DOT_POOL_ID, ACA, DOT, added_a, added_b).into(),
			Event::LiquidityAdded(ACA_DOT_POOL_ID, ACA, DOT, added_a, 0).into(),
		]);
	});
}

#[test]
fn add_liquidity_by_non_owner_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::add_liquidity(
			Origin::signed(BOB),
			ACA_DOT_POOL_ID,
			10_000_000_000,
			20_000_000_000,),
		Error::<Test>::NotOwner);
		});
}

#[test]
fn add_zero_liquidity_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::add_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, 0, 0,),
			Error::<Test>::CannotAddZeroLiquidity
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);
		assert_eq!(user_balance_b_after, user_balance_b_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn add_liquidity_insufficient_balance_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::add_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, u128::MAX, 0,),
			Error::<Test>::InsufficientAssetBalance
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);
	});
}

#[test]
fn add_liquidity_after_sale_started_should_not_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(15);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::add_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, 1_000, 1_000,),
			Error::<Test>::SaleStarted
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);
		assert_eq!(user_balance_b_after, user_balance_b_before);

		// sale ended at the block number 20
		System::set_block_number(30);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::add_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, 1_000, 1_000,),
			Error::<Test>::SaleStarted
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);
		assert_eq!(user_balance_b_after, user_balance_b_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn remove_liquidity_should_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(5);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_ok!(LBPPallet::remove_liquidity(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			1_000,
			0,
		));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before.saturating_sub(1_000));
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before.saturating_add(1_000));

		System::set_block_number(30);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		let removed_a = 10_000_000;
		let removed_b = 20_000_000;

		assert_ok!(LBPPallet::remove_liquidity(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			removed_a,
			removed_b,
		));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before.saturating_sub(removed_a));
		assert_eq!(balance_b_after, balance_b_before.saturating_sub(removed_b));

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before.saturating_add(removed_a));
		assert_eq!(user_balance_b_after, user_balance_b_before.saturating_add(removed_b));

		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_ok!(LBPPallet::remove_liquidity(
			Origin::signed(ALICE),
			ACA_DOT_POOL_ID,
			removed_a,
			0,
		));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_eq!(balance_a_after, balance_a_before.saturating_sub(removed_a));
		assert_eq!(balance_b_after, balance_b_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
			Event::LiquidityRemoved(ACA_DOT_POOL_ID, ACA, DOT, 1_000, 0).into(),
			Event::LiquidityRemoved(ACA_DOT_POOL_ID, ACA, DOT, removed_a, removed_b).into(),
			Event::LiquidityRemoved(ACA_DOT_POOL_ID, ACA, DOT, removed_a, 0).into(),
		]);
	});
}

#[test]
fn remove_liquidity_by_non_owner_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::remove_liquidity(
			Origin::signed(BOB),
			ACA_DOT_POOL_ID,
			10_000_000_000,
			20_000_000_000,),
		Error::<Test>::NotOwner);
		});
}

#[test]
fn remove_zero_liquidity_should_not_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(30);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::remove_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, 0, 0,),
			Error::<Test>::CannotRemoveZeroLiquidity
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);
		assert_eq!(user_balance_b_after, user_balance_b_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn remove_liquidity_insufficient_reserve_should_not_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(30);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::remove_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, u128::MAX, 0,),
			Error::<Test>::LiquidityUnderflow
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn remove_liquidity_during_sale_should_not_work() {
	predefined_test_ext().execute_with(|| {
		// sale started at the block number 10
		System::set_block_number(15);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::remove_liquidity(Origin::signed(ALICE), ACA_DOT_POOL_ID, 1_000, 0,),
			Error::<Test>::SaleNotEnded
		);

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, balance_a_before);
		assert_eq!(balance_b_after, balance_b_before);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		assert_eq!(user_balance_a_after, user_balance_a_before);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn destroy_pool_should_work() {
	predefined_test_ext().execute_with(|| {
		System::set_block_number(21);

		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let user_balance_hdx_before = Currency::reserved_balance(HDX, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_ok!(LBPPallet::destroy_pool(Origin::signed(ALICE), ACA_DOT_POOL_ID,));

		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);
		assert_eq!(balance_a_after, 0);
		assert_eq!(balance_b_after, 0);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		assert_eq!(
			user_balance_a_after,
			user_balance_a_before.saturating_add(balance_a_before)
		);

		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		assert_eq!(
			user_balance_b_after,
			user_balance_b_before.saturating_add(balance_b_before)
		);

		let user_balance_hdx_after = Currency::reserved_balance(HDX, &ALICE);
		assert_eq!(
			user_balance_hdx_after,
			user_balance_hdx_before.saturating_sub(POOL_DEPOSIT)
		);

		assert_eq!(<PoolOwner<Test>>::contains_key(ACA_DOT_POOL_ID), false);
		assert_eq!(<PoolDeposit<Test>>::contains_key(ACA_DOT_POOL_ID), false);
		assert_eq!(<PoolAssets<Test>>::contains_key(ACA_DOT_POOL_ID), false);
		assert_eq!(<PoolData<Test>>::contains_key(ACA_DOT_POOL_ID), false);
		assert_eq!(<PoolBalances<Test>>::contains_key(ACA_DOT_POOL_ID), false);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
			frame_system::Event::KilledAccount(ACA_DOT_POOL_ID).into(),
			Event::PoolDestroyed(ACA_DOT_POOL_ID, ACA, DOT, balance_a_before, balance_b_before).into(),
		]);
	});
}

#[test]
fn destroy_pool_by_non_owner_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::destroy_pool(
			Origin::signed(BOB),
			ACA_DOT_POOL_ID),
		Error::<Test>::NotOwner);
		});
}

#[test]
fn destroy_not_finalized_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let user_balance_a_before = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_before = Currency::free_balance(DOT, &ALICE);
		let user_balance_hdx_before = Currency::reserved_balance(HDX, &ALICE);
		let (balance_a_before, balance_b_before) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_noop!(
			LBPPallet::destroy_pool(Origin::signed(ALICE), ACA_DOT_POOL_ID,),
			Error::<Test>::SaleNotEnded
		);

		let user_balance_a_after = Currency::free_balance(ACA, &ALICE);
		let user_balance_b_after = Currency::free_balance(DOT, &ALICE);
		let user_balance_hdx_after = Currency::reserved_balance(HDX, &ALICE);
		let (balance_a_after, balance_b_after) = LBPPallet::pool_balances(ACA_DOT_POOL_ID);

		assert_eq!(balance_a_before, balance_a_after);
		assert_eq!(balance_b_before, balance_b_after);
		assert_eq!(user_balance_a_before, user_balance_a_after);
		assert_eq!(user_balance_b_before, user_balance_b_after);
		assert_eq!(user_balance_hdx_before, user_balance_hdx_after);

		expect_events(vec![
			Event::PoolCreated(ALICE, ACA_DOT_POOL_ID, ACA, DOT, 1_000_000_000, 2_000_000_000).into(),
			Event::Unpaused(ALICE, ACA_DOT_POOL_ID).into(),
		]);
	});
}

#[test]
fn execute_trade_should_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 5_000_000_u128;
		let amount_out = 10_000_000_u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_ok!(LBPPallet::execute_trade(&t));

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_998_995_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_010_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_005_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 1_990_000_000);
	});
}

// This test ensure storage was not modified on error
#[test]
fn execute_trade_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 5_000_000_u128;
		let amount_out = 10_000_000_000_000_000u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_noop!(LBPPallet::execute_trade(&t), orml_tokens::Error::<Test>::BalanceTooLow);

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);
	});
}

#[test]
fn execute_sell_should_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 8_000_000_u128;
		let amount_out = 20_000_000_u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_ok!(LBPPallet::execute_sell(&t));

		expect_events(vec![Event::SellExecuted(
			ALICE, asset_in, asset_out, amount_in, amount_out,
		)
		.into()]);

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_998_992_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_020_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_008_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 1_980_000_000);

		expect_events(vec![Event::SellExecuted(
			ALICE, asset_in, asset_out, 8_000_000, 20_000_000,
		)
		.into()]);
	});
}

// This test ensure storage was not modified on error
#[test]
fn execute_sell_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 8_000_000_000_u128;
		let amount_out = 200_000_000_000_000_u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_noop!(LBPPallet::execute_sell(&t), orml_tokens::Error::<Test>::BalanceTooLow);

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);
	});
}

#[test]
fn execute_buy_should_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 8_000_000_u128;
		let amount_out = 20_000_000_u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_ok!(LBPPallet::execute_buy(&t));

		expect_events(vec![Event::BuyExecuted(
			ALICE, asset_out, asset_in, amount_in, amount_out,
		)
		.into()]);

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_998_992_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_020_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_008_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 1_980_000_000);

		expect_events(vec![Event::BuyExecuted(
			ALICE, asset_out, asset_in, 8_000_000, 20_000_000,
		)
		.into()]);
	});
}

// This test ensure storage was not modified on error
#[test]
fn execute_buy_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		let amount_in = 8_000_000_000_u128;
		let amount_out = 200_000_000_000_000_u128;
		let t = AMMTransfer {
			origin: ALICE,
			assets: AssetPair { asset_in, asset_out },
			amount: amount_in,
			amount_out,
			discount: false,
			discount_amount: 0_u128,
		};

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);

		assert_noop!(LBPPallet::execute_buy(&t), orml_tokens::Error::<Test>::BalanceTooLow);

		assert_eq!(Currency::free_balance(asset_in, &ALICE), 999_999_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &ALICE), 999_998_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000);
	});
}

#[test]
fn sell_zero_amount_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::sell(Origin::signed(BOB), ACA, DOT, 0_u128, 200_000_u128),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn buy_zero_amount_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::buy(Origin::signed(BOB), ACA, DOT, 0_u128, 200_000_u128),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn sell_to_non_existing_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::sell(Origin::signed(BOB), ACA, ETH, 800_000_u128, 200_000_u128),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn buy_from_non_existing_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		assert_noop!(
			LBPPallet::buy(Origin::signed(BOB), ACA, ETH, 800_000_u128, 200_000_u128),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn exceed_max_in_ratio_should_not_work() {
	predefined_test_ext().execute_with(|| {
		run_to_block(11); //start sale
		assert_noop!(
			LBPPallet::sell(
				Origin::signed(BOB),
				ACA,
				DOT,
				1_000_000_000 / MAX_IN_RATIO + 1,
				200_000_u128
			),
			Error::<Test>::MaxInRatioExceeded
		);

		//1/2 should not work
		assert_noop!(
			LBPPallet::sell(Origin::signed(BOB), ACA, DOT, 1_000_000_000 / 2, 200_000_u128),
			Error::<Test>::MaxInRatioExceeded
		);

		//max ratio should work
		assert_ok!(LBPPallet::sell(
			Origin::signed(BOB),
			ACA,
			DOT,
			1_000_000_000 / MAX_IN_RATIO,
			2_000_u128
		));
	});
}

#[test]
fn exceed_max_out_ratio_should_not_work() {
	predefined_test_ext().execute_with(|| {
		run_to_block(11); //start sale

		//max_ratio_out + 1 should not work
		assert_noop!(
			LBPPallet::buy(
				Origin::signed(BOB),
				ACA,
				DOT,
				1_000_000_000 / MAX_OUT_RATIO + 1,
				200_000_u128
			),
			Error::<Test>::MaxOutRatioExceeded
		);

		//1/2 should not work
		assert_noop!(
			LBPPallet::buy(Origin::signed(BOB), ACA, DOT, 1_000_000_000 / 2, 200_000_u128),
			Error::<Test>::MaxOutRatioExceeded
		);

		//max ratio should work
		assert_ok!(LBPPallet::buy(
			Origin::signed(BOB),
			ACA,
			DOT,
			1_000_000_000 / MAX_OUT_RATIO,
			2_000_000_000_u128
		));
	});
}

#[test]
fn trade_in_non_running_pool_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = DOT;
		let amount = 800_000_u128;
		let limit = 200_000_u128;

		//sale not started
		run_to_block(9);
		assert_noop!(
			LBPPallet::sell(Origin::signed(who), asset_in, asset_out, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);
		assert_noop!(
			LBPPallet::buy(Origin::signed(who), asset_in, asset_out, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);

		//sale ended
		run_to_block(21);
		assert_noop!(
			LBPPallet::sell(Origin::signed(who), asset_in, asset_out, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);
		assert_noop!(
			LBPPallet::buy(Origin::signed(who), asset_in, asset_out, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);

		//puased pool - pool is created as puased by default
		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			ALICE,
			LBPAssetInfo {
				id: HDX,
				amount: 1_000_000_000,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: ETH,
				amount: 10_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(30u64, 40u64),
			WeightCurveType::Linear,
			true,
		));

		//pool started but is paused
		run_to_block(30);
		assert_noop!(
			LBPPallet::sell(Origin::signed(BOB), HDX, ETH, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);
		assert_noop!(
			LBPPallet::buy(Origin::signed(BOB), HDX, ETH, amount, limit),
			Error::<Test>::SaleIsNotRunning
		);
	});
}

//AssetBalanceLimitExceeded
#[test]
fn exceed_trader_limit_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = DOT;
		let amount = 800_000_u128;
		let sell_limit = 800_000_u128;
		let buy_limit = 1_000_u128;

		//start sale
		run_to_block(11);
		assert_noop!(
			LBPPallet::sell(Origin::signed(who), asset_in, asset_out, amount, sell_limit),
			Error::<Test>::AssetBalanceLimitExceeded
		);

		assert_noop!(
			LBPPallet::buy(Origin::signed(who), asset_in, asset_out, amount, buy_limit),
			Error::<Test>::AssetBalanceLimitExceeded
		);
	});
}

#[test]
fn sell_with_insufficient_balance_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = ETH;
		let amount = 800_000_u128;

		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			who,
			LBPAssetInfo {
				id: asset_in,
				amount: INITIAL_BALANCE,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: asset_out,
				amount: 10_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(30u64, 40u64),
			WeightCurveType::Linear,
			true,
		));

		//start sale
		run_to_block(31);
		assert_noop!(
			LBPPallet::sell(Origin::signed(who), asset_in, asset_out, amount, 2_u128),
			Error::<Test>::InsufficientAssetBalance
		);
	});
}

#[test]
fn buy_with_insufficient_balance_should_not_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = ETH;
		let amount = 800_000_u128;

		assert_ok!(LBPPallet::create_pool(
			Origin::root(),
			who,
			LBPAssetInfo {
				id: asset_in,
				amount: INITIAL_BALANCE,
				initial_weight: 20,
				final_weight: 90,
			},
			LBPAssetInfo {
				id: asset_out,
				amount: 10_000,
				initial_weight: 80,
				final_weight: 10,
			},
			(30u64, 40u64),
			WeightCurveType::Linear,
			true,
		));

		//start sale
		run_to_block(31);
		assert_noop!(
			LBPPallet::buy(
				Origin::signed(who),
				asset_out,
				asset_in,
				amount,
				2_000_000_000_000_000_000_000_u128
			),
			Error::<Test>::InsufficientAssetBalance
		);
	});
}

#[test]
fn buy_should_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		assert_eq!(Currency::free_balance(asset_in, &who), 1_000_000_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &who), 1_000_000_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000_u128);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000_u128);

		//start sale
		run_to_block(11);
		assert_ok!(LBPPallet::buy(
			Origin::signed(who),
			asset_out,
			asset_in,
			10_000_000_u128,
			2_000_000_000_u128
		));

		let pool = <PoolData<Test>>::get(pool_id);

		assert_eq!(
			Pool {
				start: 10u64,
				end: 20u64,
				initial_weights: ((asset_in, 20), (asset_out, 80)),
				final_weights: ((asset_in, 90), (asset_out, 10)),
				last_weight_update: 11u64,
				last_weights: ((asset_in, 27), (asset_out, 73)),
				curve: WeightCurveType::Linear,
				pausable: true,
				paused: false,
			},
			pool
		);

		assert_eq!(Currency::free_balance(asset_in, &who), 999_999_986_327_783_u128);
		assert_eq!(Currency::free_balance(asset_out, &who), 1_000_000_010_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_013_672_217_u128);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 1_990_000_000_u128);

		expect_events(vec![Event::BuyExecuted(
			who, asset_out, asset_in, 13_672_217, 10_000_000,
		)
		.into()]);
	});
}

#[test]
fn sell_should_work() {
	predefined_test_ext().execute_with(|| {
		let who = BOB;
		let asset_in = ACA;
		let asset_out = DOT;
		let pool_id = LBPPallet::get_pair_id(AssetPair { asset_in, asset_out });

		assert_eq!(Currency::free_balance(asset_in, &who), 1_000_000_000_000_000);
		assert_eq!(Currency::free_balance(asset_out, &who), 1_000_000_000_000_000);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_000_000_000_u128);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 2_000_000_000_u128);

		//start sale
		run_to_block(11);
		assert_ok!(LBPPallet::sell(
			Origin::signed(who),
			asset_in,
			asset_out,
			10_000_000_u128,
			2_000_u128
		));

		let pool = <PoolData<Test>>::get(pool_id);

		assert_eq!(
			Pool {
				start: 10u64,
				end: 20u64,
				initial_weights: ((asset_in, 20), (asset_out, 80)),
				final_weights: ((asset_in, 90), (asset_out, 10)),
				last_weight_update: 11u64,
				last_weights: ((asset_in, 27), (asset_out, 73)),
				curve: WeightCurveType::Linear,
				pausable: true,
				paused: false,
			},
			pool
		);

		assert_eq!(Currency::free_balance(asset_in, &who), INITIAL_BALANCE - 10_000_000);
		assert_eq!(Currency::free_balance(asset_out, &who), 1_000_000_007_332_274);

		assert_eq!(Currency::free_balance(asset_in, &pool_id), 1_010_000_000_u128);
		assert_eq!(Currency::free_balance(asset_out, &pool_id), 1_992_667_726_u128);

		expect_events(vec![Event::SellExecuted(
			who, asset_in, asset_out, 10_000_000, 7_332_274,
		)
		.into()]);
	});
}

#[test]
fn amm_trait_should_work() {
	predefined_test_ext().execute_with(|| {
		let asset_pair = AssetPair{asset_in: ACA, asset_out: DOT};
		let reversed_asset_pair = AssetPair{asset_in: DOT, asset_out: ACA};
		let non_existing_asset_pair = AssetPair{asset_in: DOT, asset_out: HDX};

		assert_eq!(LBPPallet::exists(asset_pair), true);
		assert_eq!(LBPPallet::exists(reversed_asset_pair), true);
		assert_eq!(LBPPallet::exists(non_existing_asset_pair), false);

		assert_eq!(LBPPallet::get_pair_id(asset_pair), ACA_DOT_POOL_ID);
		assert_eq!(LBPPallet::get_pair_id(reversed_asset_pair), ACA_DOT_POOL_ID);

		assert_eq!(LBPPallet::get_pool_assets(&ACA_DOT_POOL_ID), Some(vec![ACA, DOT]));
		assert_eq!(LBPPallet::get_pool_assets(&HDX_DOT_POOL_ID), None);

		// TODO: test all methods from the AMM trait
	});
}
