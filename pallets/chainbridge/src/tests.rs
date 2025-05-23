use frame_support::{assert_noop, assert_ok};

use super::{
	mock::{
		assert_events, new_test_ext, Balances, Bridge, ProposalLifetime, RuntimeCall, RuntimeEvent,
		RuntimeOrigin, System, Test, TestChainId, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
		TEST_THRESHOLD,
	},
	*,
};
use crate::mock::new_test_ext_initialized;

#[test]
fn derive_ids() {
	let chain = 1;
	let id = [
		0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0, 0x24,
		0xb7, 0xb1, 0x09, 0x99, 0xf4,
	];
	let r_id = derive_resource_id(chain, &id);
	let expected = [
		0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f,
		0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0, 0x24, 0xb7, 0xb1, 0x09, 0x99, 0xf4, chain,
	];
	assert_eq!(r_id, expected);
}

#[test]
fn complete_proposal_approved() {
	let mut prop = ProposalVotes {
		votes_for: vec![1, 2],
		votes_against: vec![3],
		status: ProposalStatus::Initiated,
		expiry: ProposalLifetime::get(),
	};

	prop.try_to_complete(2, 3);
	assert_eq!(prop.status, ProposalStatus::Approved);
}

#[test]
fn complete_proposal_rejected() {
	let mut prop = ProposalVotes {
		votes_for: vec![1],
		votes_against: vec![2, 3],
		status: ProposalStatus::Initiated,
		expiry: ProposalLifetime::get(),
	};

	prop.try_to_complete(2, 3);
	assert_eq!(prop.status, ProposalStatus::Rejected);
}

#[test]
fn complete_proposal_bad_threshold() {
	let mut prop = ProposalVotes {
		votes_for: vec![1, 2],
		votes_against: vec![],
		status: ProposalStatus::Initiated,
		expiry: ProposalLifetime::get(),
	};

	prop.try_to_complete(3, 2);
	assert_eq!(prop.status, ProposalStatus::Initiated);

	let mut prop = ProposalVotes {
		votes_for: vec![],
		votes_against: vec![1, 2],
		status: ProposalStatus::Initiated,
		expiry: ProposalLifetime::get(),
	};

	prop.try_to_complete(3, 2);
	assert_eq!(prop.status, ProposalStatus::Initiated);
}

#[test]
fn setup_resources() {
	new_test_ext().execute_with(|| {
		let id: ResourceId = [1; 32];
		let method = "Pallet.do_something".as_bytes().to_vec();
		let method2 = "Pallet.do_somethingElse".as_bytes().to_vec();

		assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), id, method.clone()));
		assert_eq!(Resources::<Test>::get(id), Some(method));

		assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), id, method2.clone()));
		assert_eq!(Resources::<Test>::get(id), Some(method2));

		assert_ok!(Bridge::remove_resource(RuntimeOrigin::root(), id));
		assert_eq!(Resources::<Test>::get(id), None);
	})
}

#[test]
fn whitelist_chain() {
	new_test_ext().execute_with(|| {
		assert!(!Bridge::chain_whitelisted(0));

		assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), 0));
		assert_noop!(
			Bridge::whitelist_chain(RuntimeOrigin::root(), TestChainId::get()),
			Error::<Test>::InvalidChainId
		);

		assert_events(vec![(Event::ChainWhitelisted(0)).into()]);
	})
}

#[test]
fn set_get_threshold() {
	new_test_ext().execute_with(|| {
		assert_eq!(<RelayerThreshold<Test>>::get(), 1);

		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), TEST_THRESHOLD));
		assert_eq!(<RelayerThreshold<Test>>::get(), TEST_THRESHOLD);

		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), 5));
		assert_eq!(<RelayerThreshold<Test>>::get(), 5);

		assert_events(vec![
			RuntimeEvent::Bridge(Event::RelayerThresholdChanged(TEST_THRESHOLD)),
			RuntimeEvent::Bridge(Event::RelayerThresholdChanged(5)),
		]);
	})
}

#[test]
fn asset_transfer_success() {
	new_test_ext().execute_with(|| {
		let dest_id = 2;
		let to = vec![2];
		let resource_id = [1; 32];
		let metadata = vec![];
		let amount = 100;
		let token_id = vec![1, 2, 3, 4];
		let method = "Erc20.transfer".as_bytes().to_vec();

		assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), resource_id, method));
		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), TEST_THRESHOLD,));

		assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), dest_id));
		assert_ok!(Bridge::transfer_fungible(dest_id, resource_id, to.clone(), amount.into()));
		assert_events(vec![
			RuntimeEvent::Bridge(Event::ChainWhitelisted(dest_id)),
			RuntimeEvent::Bridge(Event::FungibleTransfer(
				dest_id,
				1,
				resource_id,
				amount.into(),
				to.clone(),
			)),
		]);

		assert_ok!(Bridge::transfer_nonfungible(
			dest_id,
			resource_id,
			token_id.clone(),
			to.clone(),
			metadata.clone()
		));
		assert_events(vec![Event::NonFungibleTransfer(
			dest_id,
			2,
			resource_id,
			token_id,
			to,
			metadata.clone(),
		)
		.into()]);

		assert_ok!(Bridge::transfer_generic(dest_id, resource_id, metadata.clone()));
		assert_events(vec![Event::GenericTransfer(dest_id, 3, resource_id, metadata).into()]);
	})
}

#[test]
fn asset_transfer_invalid_resource_id() {
	new_test_ext().execute_with(|| {
		let dest_id = 2;
		let to = vec![2];
		let resource_id = [1; 32];
		let amount = 100;

		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), TEST_THRESHOLD,));
		assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), dest_id));

		assert_noop!(
			Bridge::transfer_fungible(dest_id, resource_id, to, amount.into()),
			Error::<Test>::ResourceDoesNotExist
		);

		assert_noop!(
			Bridge::transfer_nonfungible(dest_id, resource_id, vec![], vec![], vec![]),
			Error::<Test>::ResourceDoesNotExist
		);

		assert_noop!(
			Bridge::transfer_generic(dest_id, resource_id, vec![]),
			Error::<Test>::ResourceDoesNotExist
		);
	})
}

#[test]
fn asset_transfer_invalid_chain() {
	new_test_ext().execute_with(|| {
		let chain_id = 2;
		let bad_dest_id = 3;
		let resource_id = [4; 32];

		assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), chain_id));
		assert_events(vec![Event::ChainWhitelisted(chain_id).into()]);

		assert_noop!(
			Bridge::transfer_fungible(bad_dest_id, resource_id, vec![], U256::zero()),
			Error::<Test>::ChainNotWhitelisted
		);

		assert_noop!(
			Bridge::transfer_nonfungible(bad_dest_id, resource_id, vec![], vec![], vec![]),
			Error::<Test>::ChainNotWhitelisted
		);

		assert_noop!(
			Bridge::transfer_generic(bad_dest_id, resource_id, vec![]),
			Error::<Test>::ChainNotWhitelisted
		);
	})
}

#[test]
fn add_remove_relayer() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), TEST_THRESHOLD,));
		assert_eq!(RelayerCount::<Test>::get(), 0);

		assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_A));
		assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_B));
		assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_C));
		assert_eq!(RelayerCount::<Test>::get(), 3);

		// Already exists
		assert_noop!(
			Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_A),
			Error::<Test>::RelayerAlreadyExists
		);

		// Confirm removal
		assert_ok!(Bridge::remove_relayer(RuntimeOrigin::root(), RELAYER_B));
		assert_eq!(RelayerCount::<Test>::get(), 2);
		assert_noop!(
			Bridge::remove_relayer(RuntimeOrigin::root(), RELAYER_B),
			Error::<Test>::RelayerInvalid
		);
		assert_eq!(RelayerCount::<Test>::get(), 2);

		assert_events(vec![
			RuntimeEvent::Bridge(Event::RelayerAdded(RELAYER_A)),
			RuntimeEvent::Bridge(Event::RelayerAdded(RELAYER_B)),
			RuntimeEvent::Bridge(Event::RelayerAdded(RELAYER_C)),
			RuntimeEvent::Bridge(Event::RelayerRemoved(RELAYER_B)),
		]);
	})
}

fn make_proposal(r: Vec<u8>) -> mock::RuntimeCall {
	RuntimeCall::System(system::Call::remark { remark: r })
}

#[test]
fn create_sucessful_proposal() {
	let src_id = 1;
	let r_id = derive_resource_id(src_id, b"remark");

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_proposal(vec![10]);

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Second relayer votes against
		assert_ok!(Bridge::reject_proposal(
			RuntimeOrigin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![RELAYER_B],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Third relayer votes in favour
		assert_ok!(Bridge::acknowledge_proposal(
			RuntimeOrigin::signed(RELAYER_C),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal)).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A, RELAYER_C],
			votes_against: vec![RELAYER_B],
			status: ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_events(vec![
			RuntimeEvent::Bridge(Event::VoteFor(src_id, prop_id, RELAYER_A)),
			RuntimeEvent::Bridge(Event::VoteAgainst(src_id, prop_id, RELAYER_B)),
			RuntimeEvent::Bridge(Event::VoteFor(src_id, prop_id, RELAYER_C)),
			RuntimeEvent::Bridge(Event::ProposalApproved(src_id, prop_id)),
			RuntimeEvent::Bridge(Event::ProposalSucceeded(src_id, prop_id)),
		]);
	})
}

#[test]
fn create_unsucessful_proposal() {
	let src_id = 1;
	let r_id = derive_resource_id(src_id, b"transfer");

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_proposal(vec![11]);

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Second relayer votes against
		assert_ok!(Bridge::reject_proposal(
			RuntimeOrigin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![RELAYER_B],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Third relayer votes against
		assert_ok!(Bridge::reject_proposal(
			RuntimeOrigin::signed(RELAYER_C),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal)).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![RELAYER_B, RELAYER_C],
			status: ProposalStatus::Rejected,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_eq!(Balances::free_balance(RELAYER_B), 0);
		assert_eq!(Balances::free_balance(Bridge::account_id()), ENDOWED_BALANCE);

		assert_events(vec![
			RuntimeEvent::Bridge(Event::VoteFor(src_id, prop_id, RELAYER_A)),
			RuntimeEvent::Bridge(Event::VoteAgainst(src_id, prop_id, RELAYER_B)),
			RuntimeEvent::Bridge(Event::VoteAgainst(src_id, prop_id, RELAYER_C)),
			RuntimeEvent::Bridge(Event::ProposalRejected(src_id, prop_id)),
		]);
	})
}

#[test]
fn execute_after_threshold_change() {
	let src_id = 1;
	let r_id = derive_resource_id(src_id, b"transfer");

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_proposal(vec![11]);

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Change threshold
		assert_ok!(Bridge::set_threshold(RuntimeOrigin::root(), 1));

		// Attempt to execute
		assert_ok!(Bridge::eval_vote_state(
			RuntimeOrigin::signed(RELAYER_A),
			prop_id,
			src_id,
			Box::new(proposal.clone())
		));

		let prop = Votes::<Test>::get(src_id, (prop_id, proposal)).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_eq!(Balances::free_balance(RELAYER_B), 0);
		assert_eq!(Balances::free_balance(Bridge::account_id()), ENDOWED_BALANCE);

		assert_events(vec![
			RuntimeEvent::Bridge(Event::VoteFor(src_id, prop_id, RELAYER_A)),
			RuntimeEvent::Bridge(Event::RelayerThresholdChanged(1)),
			RuntimeEvent::Bridge(Event::ProposalApproved(src_id, prop_id)),
			RuntimeEvent::Bridge(Event::ProposalSucceeded(src_id, prop_id)),
		]);
	})
}

#[test]
fn proposal_expires() {
	let src_id = 1;
	let r_id = derive_resource_id(src_id, b"remark");

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_proposal(vec![10]);

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Increment enough blocks such that now == expiry
		System::set_block_number(ProposalLifetime::get() + 1);

		// Attempt to submit a vote should fail
		assert_noop!(
			Bridge::reject_proposal(
				RuntimeOrigin::signed(RELAYER_B),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone())
			),
			Error::<Test>::ProposalExpired
		);

		// Proposal state should remain unchanged
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// eval_vote_state should have no effect
		assert_noop!(
			Bridge::eval_vote_state(
				RuntimeOrigin::signed(RELAYER_C),
				prop_id,
				src_id,
				Box::new(proposal.clone())
			),
			Error::<Test>::ProposalExpired
		);
		let prop = Votes::<Test>::get(src_id, (prop_id, proposal)).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_events(vec![Event::VoteFor(src_id, prop_id, RELAYER_A).into()]);
	})
}
