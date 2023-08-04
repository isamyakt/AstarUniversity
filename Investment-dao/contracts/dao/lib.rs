#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod dao {
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
        ProposalNotAccepted
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
        pub vote_start: Timestamp,
        pub vote_end: Timestamp,
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
        pub for_votes: u64,
        pub against_votes: u64,
    }

    pub type ProposalId = u32;

    #[ink(storage)]
    pub struct Governor {
        // to implement
        proposals: Mapping<ProposalId, Proposal>,
        proposal_votes: Mapping<ProposalId, ProposalVote>,
        votes: Mapping<AccountId, ProposalId>,
        next_proposal_id: ProposalId,
        quorum: u8,
        governance_token: AccountId
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            Self { 
                proposals: Mapping::new(), 
                proposal_votes: Mapping::new(), 
                votes: Mapping::new(), 
                next_proposal_id: 0, 
                quorum, 
                governance_token
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

            let proposal = Proposal {
                to,
                amount,
                vote_start: now,
                vote_end: now + duration,
                executed: false
            };

            let proposal_id = self.next_proposal_id;
            self.proposals.insert(proposal_id, &proposal);

            self.next_proposal_id += 1;

            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            let sender = self.env().caller();

            let proposal = match self.proposals.get(&proposal_id) {
                Some(proposal) => proposal,
                None => return Err(GovernorError::ProposalNotFound),
            };

            if proposal.executed {
                return Err(GovernorError::ProposalAlreadyExecuted);
            }

            let now = self.env().block_timestamp();

            if now >= proposal.vote_end {
                return Err(GovernorError::VotePeriodEnded);
            }

            if self.votes.contains(&sender) {
                return Err(GovernorError::AlreadyVoted);
            }

            let mut proposal_vote = self.proposal_votes.get(&proposal_id).unwrap_or_default();

            match vote {
                VoteType::Aganist => {proposal_vote.against_votes += 1},
                VoteType::For => {proposal_vote.for_votes += 1}
            }

            self.proposal_votes.insert(&proposal_id, &proposal_vote);
            
            self.votes.insert(sender, &proposal_id);

            Ok(())
        }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
            let mut proposal = match self.proposals.get(&proposal_id) {
                Some(proposal) => proposal,
                None => return Err(GovernorError::ProposalNotFound)
            };

            if proposal.executed {
                return Err(GovernorError::ProposalNotFound);
            }

            let now = self.env().block_timestamp();

            if now < proposal.vote_end {
                return Err(GovernorError::QuorumNotReached);
            }

            let proposal_vote = match self.proposal_votes.get(&proposal_id) {
                Some(proposal_vote) => proposal_vote,
                None => return Err(GovernorError::ProposalNotFound)
            };

            // let total_votes = proposal_vote.for_votes + proposal_vote.against_votes;

            if proposal_vote.for_votes <= proposal_vote.against_votes {
                return Err(GovernorError::ProposalNotAccepted);
            }

            proposal.executed = true;
            self.proposals.insert(proposal_id, &proposal);

            self.env().transfer(proposal.to, proposal.amount)
                .expect("failed to transfer funds");
            
            Ok(())
        }

        // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }

        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: ProposalId) -> Proposal {
            self.proposals.get(proposal_id).unwrap()
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
            let proposal = governor.get_proposal(0);
            let now = governor.now();
            const ONE_MINUTE: u64 = 1;
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
            assert_eq!(governor.next_proposal_id, 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let result = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(result, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::QuorumNotReached));
        }
    }
}