#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(PSP22, PSP22Mintable,PSP22Metadata)]
#[openbrush::contract]
pub mod governance_token {
    use openbrush::{traits::Storage, contracts::traits::psp22::extensions::{wrapper::psp22, metadata}};

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct GovernanceToken {
    	#[storage_field]
		psp22: psp22::Data,
		#[storage_field]
		metadata: metadata::Data,
    }
    
    impl GovernanceToken {
        #[ink(constructor)]
        pub fn new(initial_supply: Balance, name: Option<String>, symbol: Option<String>, decimal: u8) -> Self {
            let mut _instance = Self::default();
			psp22::Internal::_mint(&mut _instance, Self::env().caller(), initial_supply).expect("Should mint"); 
			_instance.metadata.name.set(&name);
			_instance.metadata.symbol.set(&symbol);
			_instance.metadata.decimals.set(&decimal);
			_instance
        }
    }
}
