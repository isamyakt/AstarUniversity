#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod dao {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;
    use ink::storage::Mapping;
    use scale::{
        Decode,
        Encode,
    };

    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo))]
    pub enum VoteType {
        // to implement
        For,
        Aganist
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError {
        // to implement
        AmountShouldNotBeZero,
        DurationError,
        QuorumNotReached,
        ProposalNotFound,
        ProposalAlreadyExecuted,
        VotePeriodEnded,
        AlreadyVoted,
        ProposalNotAccepted,
        TransactionFailed
    }

    #[derive(Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct Proposal {
        // to implement
        pub to: AccountId,
        pub amount: Balance,
        pub vote_start: u64,
        pub vote_end: u64,
        pub executed: bool,
    }

    #[derive(Encode, Decode, Default)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct ProposalVote {
        // to implement
        pub for_votes: u128,
        pub against_votes: u128,
    }

    pub type ProposalId = u32;
    const ONE_MINUTE: u64 = 60;

    #[ink(storage)]
    pub struct Governor {
        // to implement
        proposals: Mapping<ProposalId, Proposal>,
        proposal_votes: Mapping<Proposal, ProposalVote>,
        votes: Mapping<(ProposalId, AccountId), ()>,
        next_proposal_id: ProposalId,
        quorum: u8,
        governance_token: AccountId
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            Self { 
                proposals: Default::default(),
                proposal_votes: Default::default(),
                votes: Default::default(),
                next_proposal_id: Default::default(),
                quorum,
                governance_token,
            }
        }

        #[ink(message)]
        pub fn propose(
            &mut self,
            to: AccountId,
            amount: Balance,
            duration: u64,
        ) -> Result<(), GovernorError> {

            if amount <= 0 {
                return Err(GovernorError::AmountShouldNotBeZero);
            }
            if duration <= 0 {
                return Err(GovernorError::DurationError);
            }

            let now = self.env().block_timestamp();

            let prop = Proposal {
                to,
                amount,
                vote_start: now,
                vote_end: now + duration * ONE_MINUTE,
                executed: false
            };

            self.next_proposal_id = self.next_proposal_id() + 1;
            self.proposals.insert(self.next_proposal_id, &prop);
            self.proposal_votes.insert(prop, &{ProposalVote {
                for_votes: 0,
                against_votes: 0
            }});

            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            let sender = self.env().caller();

            if self.proposals.contains(&proposal_id) {
                return Err(GovernorError::ProposalNotFound)
            };

            match self.get_proposal(proposal_id.clone()) {
                None => {}
                Some(p) => {
                    if p.executed == true {
                        return Err(GovernorError::ProposalAlreadyExecuted)
                    }

                    if p.vote_end < self.env().block_timestamp() {
                        return Err(GovernorError::VotePeriodEnded)
                    }
                }
            }

            if self.votes.contains(&(proposal_id, sender)) {
                return Err(GovernorError::AlreadyVoted);
            }

            self.votes.insert(&(proposal_id, sender), &());

            let caller_balance = self.balance_of_acc(sender);
            let total_balance = self.get_total_supply();
            let votes_weight = caller_balance / total_balance * 100;
            let proposal = self.get_proposal(proposal_id).unwrap();
            let mut proposal_vote = self.proposal_votes.get(&proposal).expect("not found");

            match vote {
                VoteType::Aganist => {proposal_vote.against_votes += votes_weight},
                VoteType::For => {proposal_vote.for_votes += votes_weight}
            }
            
            self.proposal_votes.insert(proposal, &proposal_vote);

            Ok(())
        }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
            if self.proposals.contains(&proposal_id) {
                return Err(GovernorError::ProposalNotFound);
            };

            let mut proposal = self.get_proposal(proposal_id).unwrap();
            if proposal.executed == true {
                return Err(GovernorError::ProposalAlreadyExecuted)
            }

            let now = self.env().block_timestamp();

            if now < proposal.vote_end {
                return Err(GovernorError::QuorumNotReached);
            }

            if let Some(votes) = self.get_proposal_votes(proposal_id) {
                if votes.against_votes + votes.for_votes < self.quorum.into() {
                    return Err(GovernorError::QuorumNotReached);
                }

                if votes.against_votes < votes.for_votes {
                    return Err(GovernorError::ProposalNotAccepted);
                }
            }

            proposal.executed = true;
            
            build_call::<DefaultEnvironment>()
                .call(self.governance_token)
                .gas_limit(5_000_000_000)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP22::transfer"
                    )))
                        .push_arg(proposal.to)
                        .push_arg(proposal.amount),
                )
                .returns::<()>()
                .try_invoke()
                .map_err(|_| GovernorError::TransactionFailed)?
                .map_err(|_| GovernorError::TransactionFailed)?;

            
            Ok(())
        }

        // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }

        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<Proposal> {
            if let Some(prop) = self.proposals.get(proposal_id) {
                Some(prop)
            } else {
                None
            }
        }

        #[ink(message)]
        pub fn next_proposal_id(&self) -> ProposalId  {
            self.next_proposal_id
        }

        fn get_proposal_votes(&self, proposal_id: ProposalId) -> Option<ProposalVote> {
            let prop = self.get_proposal(proposal_id).unwrap();
            if let Some(votes_distribution) = self.proposal_votes.get(&prop) {
                Some(votes_distribution)
            } else {
                None
            }
        }

        fn balance_of_acc(&self, account_id: AccountId) -> Balance {
            build_call::<DefaultEnvironment>()
                .call(self.governance_token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("balance_of")))
                        .push_arg(&account_id)
                )
                .returns::<Balance>()
                .invoke()
        }

        fn get_total_supply(&self) -> Balance {
            build_call::<DefaultEnvironment>()
                .call(self.governance_token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("total_supply")))
                )
                .returns::<Balance>()
                .invoke()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_contract(initial_balance: Balance) -> Governor {
            let accounts = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            Governor::new(AccountId::from([0x01; 32]), 50)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        #[ink::test]
        fn propose_works() {
            let accounts = default_accounts();
            let mut governor = create_contract(1000);
            assert_eq!(
                governor.propose(accounts.django, 0, 1),
                Err(GovernorError::AmountShouldNotBeZero)
            );
            assert_eq!(
                governor.propose(accounts.django, 100, 0),
                Err(GovernorError::DurationError)
            );
            let result = governor.propose(accounts.django, 100, 1);
            assert_eq!(result, Ok(()));
            let proposal = governor.get_proposal(1).unwrap();
            let now = governor.now();
            assert_eq!(
                proposal,
                Proposal {
                    to: accounts.django,
                    amount: 100,
                    vote_start: 0,
                    vote_end: now + 1 * ONE_MINUTE,
                    executed: false,
                }
            );
            assert_eq!(governor.next_proposal_id(), 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let result = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(result, Ok(()));
            assert_eq!(governor.next_proposal_id(), 1);
            let execute = governor.execute(1);
            assert_eq!(execute, Err(GovernorError::ProposalNotFound));
        }
    }
}