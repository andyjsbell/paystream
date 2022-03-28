use near_contract_standards::fungible_token::{
    core::FungibleTokenCore,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC},
    resolver::FungibleTokenResolver,
    FungibleToken,
};
use near_sdk::{json_types::U128};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    PromiseOrValue,
};
use near_sdk::{
    collections::{LazyOption, LookupMap},
    PanicOnDefault,
};
use near_sdk::{env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, Promise};
use near_sdk::{ext_contract, PromiseResult};

// https://stackoverflow.com/questions/69096013/how-can-i-serialize-a-near-sdk-rs-lookupmap-that-uses-a-string-as-a-key-or-is-t
#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    FungibleToken,
    Metadata,
    Balances,
    Subscriptions,
    Outputs,
    Inputs,
}

type SubscriptionIndex = u64;
type YoctoPerSecond = u128;

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")] 
pub struct Subscription {
    source: AccountId,
    destination: AccountId,
    rate: YoctoPerSecond,
    timestamp: u64,
}

impl Subscription {
    /// Settle the subscription returning the amount to settle
    pub fn settle(&mut self) -> Balance {
        let timestamp = env::block_timestamp();
        let time_spent = timestamp.saturating_sub(self.timestamp);
        let amount = (time_spent as u128).saturating_mul(self.rate);
        self.timestamp = timestamp;
        amount
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Subscriptions {
    /// Index of current subscription
    pub subscription_index: SubscriptionIndex,
    /// The subscriptions
    pub subscriptions: LookupMap<SubscriptionIndex, Subscription>,
    /// Outputs
    pub outputs: LookupMap<AccountId, Vec<SubscriptionIndex>>,
    /// Inputs
    pub inputs: LookupMap<AccountId, Vec<SubscriptionIndex>>,
}

impl Subscriptions {
    pub fn create(
        &mut self,
        source: AccountId,
        destination: AccountId,
        rate: YoctoPerSecond,
    ) -> Subscription {
        self.subscription_index = self.subscription_index.wrapping_add(1);

        let subscription = Subscription {
            source: source.clone(),
            destination: destination.clone(),
            rate,
            timestamp: env::block_timestamp(),
        };
        self.subscriptions
            .insert(&self.subscription_index, &subscription);

        let mut inputs = self.inputs.get(&destination).unwrap_or_default();
        inputs.push(self.subscription_index);
        self.inputs.insert(&destination, &inputs);

        let mut outputs = self.outputs.get(&source).unwrap_or_default();
        outputs.push(self.subscription_index);
        self.outputs.insert(&source, &outputs);

        subscription
    }

    pub fn exists(&self, subscription_index: SubscriptionIndex) -> bool {
        self.subscriptions.contains_key(&subscription_index)
    }

    pub fn try_get(
        &self,
        subscription_index: SubscriptionIndex,
    ) -> Result<Subscription, &'static str> {
        self.subscriptions
            .get(&subscription_index)
            .ok_or("subscription not present")
    }

    pub fn try_remove(
        &mut self,
        subscription_index: SubscriptionIndex,
    ) -> Result<Subscription, &'static str> {
        let subscription = self
            .subscriptions
            .remove(&subscription_index)
            .ok_or("subscription not present")?;

        if let Some(mut inputs) = self.inputs.get(&subscription.source) {
            inputs.retain(|&input| input == subscription_index);
            self.inputs.insert(&subscription.source, &inputs);
        }

        if let Some(mut outputs) = self.outputs.get(&subscription.destination) {
            outputs.retain(|&output| output == subscription_index);
            self.outputs.insert(&subscription.source, &outputs);
        }

        Ok(subscription)
    }

    pub fn indices(&self, account_id: AccountId) -> Vec<SubscriptionIndex> {
        let mut inputs = self.inputs.get(&account_id).unwrap_or_default();
        let mut outputs = self.outputs.get(&account_id).unwrap_or_default();
        inputs.append(&mut outputs);
        inputs
    }

    pub fn try_index(
        &self,
        subscription_index: SubscriptionIndex,
    ) -> Result<Subscription, &'static str> {
        self.subscriptions
            .get(&subscription_index)
            .ok_or("subscription not present")
    }

    fn try_update(
        &mut self,
        subscription_index: SubscriptionIndex,
        new_flow: YoctoPerSecond,
    ) -> Result<Subscription, &'static str> {
        let mut subscription = self.try_get(subscription_index)?;
        if subscription.rate == new_flow {
            return Err("flow must be different");
        }
        subscription.rate = new_flow;
        self.subscriptions
            .insert(&self.subscription_index, &subscription)
            .ok_or("unable to update subscription")?;
        Ok(subscription)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Paystream {
    wrap_contract: AccountId,
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    /// Balances of streams
    balances: LookupMap<AccountId, Balance>,
    /// The owner of the account
    owner: AccountId,
    /// The treasury controlling account
    /// wNEAR would be assigned to this account
    treasurer: AccountId,
    /// Subscriptions
    subscriptions: Subscriptions,
}

// sNEAR fungible token
// We wrap wNEAR so you could say a wrap of a wrapper
// Over the testnet we call `wrap`
const WRAP_CONTRACT: &str = "wrap.testnet";
const STREAM_SYMBOL: &str = "STREAM";
const STREAM_NAME: &str = "sNEAR fungible token";
const DECIMALS: u8 = 24;
// TODO change this symbol
const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[ext_contract(ext_ft)]
pub trait FungibleToken {
    fn ft_balance_of(&mut self, account_id: AccountId) -> U128;
}

#[ext_contract(ext_wnear)]
pub trait wNear {
    #[payable]
    fn near_deposit(&mut self);
}

#[ext_contract(ext_self)]
pub trait Callbacks {
    fn wrap_callback(&mut self, account_id: AccountId, amount: Balance);
}

trait Permission {
    fn required(account_id: &AccountId);
}

impl Permission for Paystream {
    fn required(account_id: &AccountId) {
        require!(
            &env::signer_account_id() == account_id,
            "Permission required"
        );
    }
}

// Owner control
#[near_bindgen]
impl Paystream {
    /// Return who the owner of the contract is
    pub fn owner(&self) -> &AccountId {
        &self.owner
    }

    /// Owner can be set only by `owner`
    pub fn set_owner(&mut self, new_owner: AccountId) {
        Self::required(self.owner());
        require!(&new_owner != self.owner(), "should be new owner");
        self.owner = new_owner;
    }
}

// Treasury control
#[near_bindgen]
impl Paystream {
    /// Return who the treasurer of the contract is
    pub fn treasurer(&self) -> &AccountId {
        &self.treasurer
    }

    /// Treasurer can be set only by `owner`
    pub fn set_treasurer(&mut self, new_treasurer: AccountId) {
        Self::required(self.owner());
        require!(
            &new_treasurer != self.treasurer(),
            "should be new treasurer"
        );
        self.treasurer = new_treasurer;
    }
}

// Subscriptions
#[near_bindgen]
impl Paystream {
    pub fn add_subscription(
        &mut self,
        source: AccountId,
        destination: AccountId,
        rate: YoctoPerSecond,
    ) -> Subscription {
        require!(rate > 0, "rate needs to be greater than zero");
        require!(source != destination, "source must not be destination");

        self.subscriptions.create(source, destination, rate)
    }

    pub fn remove_subscription(&mut self, subscription_index: SubscriptionIndex) -> Subscription {
        let subscription = self.subscriptions.try_get(subscription_index).unwrap();
        require!(
            subscription.source == env::signer_account_id()
                || subscription.destination == env::signer_account_id(),
            "signer must be source or destination"
        );

        let mut subscription = self
            .subscriptions
            .try_remove(subscription_index)
            .expect("subscription is removed");

        let amount = subscription.settle();
        self.try_transfer(
            subscription.source.clone(),
            subscription.destination.clone(),
            amount,
        )
        .expect("transfer on settlement");

        subscription
    }

    pub fn subscriptions_by_account(&self) -> Vec<SubscriptionIndex> {
        self.subscriptions.indices(env::signer_account_id())
    }

    pub fn get_subscription(&self, subscription_index: SubscriptionIndex) -> Subscription {
        self.subscriptions.try_get(subscription_index).unwrap()
    }

    pub fn update_subscription(
        &mut self,
        subscription_index: SubscriptionIndex,
        new_flow: YoctoPerSecond,
    ) -> Subscription {
        let mut subscription = self.subscriptions.try_get(subscription_index).unwrap();
        let amount = subscription.settle();
        self.try_transfer(subscription.source, subscription.destination, amount).unwrap();
        self.subscriptions
            .try_update(subscription_index, new_flow)
            .unwrap()
    }
}

#[near_bindgen]
impl Paystream {
    #[init]
    pub fn new(owner: AccountId, wrap_contract: AccountId) -> Self {
        require!(!env::state_exists(), "Already initialized");
        // Metadata for the wrapped wrapper
        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.into(),
            name: STREAM_NAME.into(),
            symbol: STREAM_SYMBOL.into(),
            icon: Some(DATA_IMAGE_SVG_NEAR_ICON.into()),
            reference: None,
            reference_hash: None,
            decimals: DECIMALS,
        };
        metadata.assert_valid();

        // Initialise contract
        let mut this = Self {
            wrap_contract,
            balances: LookupMap::new(StorageKey::Balances),
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            owner: owner.clone(),
            treasurer: owner.clone(),
            subscriptions: Subscriptions {
                subscription_index: 0,
                subscriptions: LookupMap::new(StorageKey::Subscriptions),
                outputs: LookupMap::new(StorageKey::Outputs),
                inputs: LookupMap::new(StorageKey::Inputs),
            },
        };

        this.token.internal_register_account(&owner);
        // No initial supply
        this.token.internal_deposit(&owner, 0);
        this
    }

    /// Wrap NEAR as wNEAR as a cross contract call and on success credit the
    /// account's balance as sNEAR
    #[payable]
    pub fn wrap_near(&mut self) -> Promise {
        ext_wnear::near_deposit(
            WRAP_CONTRACT.parse().unwrap(),
            env::attached_deposit(),
            5_000_000_000_000u64.into(),
        )
        .then(ext_self::wrap_callback(
            env::signer_account_id(),
            env::attached_deposit(),
            env::current_account_id(),
            0,
            5_000_000_000_000u64.into(),
        ))
    }

    /// Unwrap wNEAR and credit the signer the amount in NEAR
    #[payable]
    pub fn unwrap_near(&mut self, _amount: Balance) -> Promise {
        Promise::new(env::current_account_id())
    }

    pub fn wrap_callback(&mut self, account_id: AccountId, amount: Balance) {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            // TODO what to be done if the cross contract fails
            PromiseResult::Failed => log!("failed callback"),
            // Near has been wrapped, update balance of sNEAR for account
            PromiseResult::Successful(_) => {
                match self.balances.get(&account_id) {
                    Some(current_balance) => self
                        .balances
                        .insert(&account_id, &current_balance.saturating_add(amount)),
                    None => self.balances.insert(&account_id, &amount),
                };
            }
        }
    }
}

impl Paystream {
    fn try_transfer(
        &mut self,
        source: AccountId,
        destination: AccountId,
        amount: Balance,
    ) -> Result<(), &'static str> {
        let balance_of_source = self.balances.get(&source).ok_or("source doesn't exist")?;
        let new_balance_of_source = balance_of_source
            .checked_sub(amount)
            .ok_or("insufficient balance")?;

        self.balances.insert(&source, &new_balance_of_source);

        match self.balances.get(&destination) {
            Some(current_balance) => self
                .balances
                .insert(&destination, &current_balance.saturating_add(amount)),
            None => self.balances.insert(&destination, &amount),
        };

        Ok(())
    }

    fn current_balance(&self, account_id: AccountId) -> U128 {
        let mut balance = self.balances.get(&account_id).unwrap_or_default();
        // All incoming where account is destination
        let timestamp = env::block_timestamp();

        // TODO Naming could be better here
        let yoctos_per_second = |subscription: &Subscription| -> u128 {
            let difference = timestamp.saturating_sub(subscription.timestamp);
            (difference as u128).saturating_mul(subscription.rate)
        };

        self.subscriptions
            .inputs
            .get(&account_id)
            .unwrap_or_default()
            .iter()
            .for_each(|subscription_index| {
                if let Ok(subscription) = self.subscriptions.try_get(*subscription_index) {
                    balance = balance.saturating_add(yoctos_per_second(&subscription));
                }
            });

        // All outgoing where account is source
        self.subscriptions
            .outputs
            .get(&account_id)
            .unwrap_or_default()
            .iter()
            .for_each(|subscription_index| {
                if let Ok(subscription) = self.subscriptions.try_get(*subscription_index) {
                    // TODO check here the reserve amount??  Maybe it won't matter but to be sure
                    balance = balance.saturating_sub(yoctos_per_second(&subscription));
                }
            });

        balance.into()
    }
}

#[near_bindgen]
impl FungibleTokenCore for Paystream {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.current_balance(account_id)
    }
}

// Handlers for `FungibleTokenResolver`
impl Paystream {
    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Paystream {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            self.on_tokens_burned(sender_id, burned_amount);
        }
        used_amount.into()
    }
}

near_contract_standards::impl_fungible_token_storage!(Paystream, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Paystream {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_contract_standards::fungible_token::core::FungibleTokenCore;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, 0);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, 0);
        assert_eq!(contract.owner(), &accounts(0));
    }

    #[test]
    #[should_panic(expected = "Permission required")]
    fn test_signer_is_owner() {
        let mut context = get_context(accounts(1));
        Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        testing_env!(context
            .is_view(false)
            .signer_account_id(accounts(0))
            .build());
        Paystream::required(&accounts(0));
        Paystream::required(&accounts(1));
    }

    #[test]
    #[should_panic(expected = "should be new owner")]
    fn test_should_be_new_owner() {
        let mut context = get_context(accounts(1));
        let mut paystream = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        testing_env!(context
            .is_view(false)
            .signer_account_id(accounts(0))
            .build());
        paystream.set_owner(accounts(1));
        testing_env!(context
            .is_view(false)
            .signer_account_id(accounts(1))
            .build());
        paystream.set_owner(accounts(1));
    }

    #[test]
    #[should_panic(expected = "should be new treasurer")]
    fn test_should_be_new_treasurer() {
        let mut context = get_context(accounts(1));
        let mut paystream = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        testing_env!(context
            .is_view(false)
            .signer_account_id(accounts(0))
            .build());
        paystream.set_treasurer(accounts(1));
        paystream.set_treasurer(accounts(1));
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Paystream::default();
    }

    #[test]
    #[should_panic(expected = "subscription not present")]
    fn test_livecycle_of_subscription() {
        let mut context = get_context(accounts(1));
        let block_timestamp = 10;
        let rate = 100;
        testing_env!(context.block_timestamp(block_timestamp).build());
        let mut contract = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        let subscription = contract.add_subscription(accounts(1), accounts(2), rate);
        assert_eq!(subscription.source, accounts(1));
        assert_eq!(subscription.destination, accounts(2));
        assert_eq!(subscription.rate, rate);
        assert_eq!(subscription.timestamp, block_timestamp);

        let subscriptions = contract.subscriptions_by_account();
        let new_subscription = contract.get_subscription(subscriptions[0]);
        assert_eq!(
            new_subscription, subscription,
            "what is created isn't what is stored"
        );

        let updated_subscription = contract.update_subscription(subscriptions[0], 200);
        assert_eq!(
            updated_subscription.rate, 200,
            "rate should have been updated"
        );

        contract.remove_subscription(subscriptions[0]);
        contract.get_subscription(subscriptions[0]);
    }

    #[test]
    #[should_panic(expected = "source must not be destination")]
    fn test_should_fail_source_must_not_be_destination() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        contract.add_subscription(accounts(1), accounts(1), 100);
    }

    #[test]
    #[should_panic(expected = "rate needs to be greater than zero")]
    fn test_rate_needs_to_be_greater_than_zero() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Paystream::new(accounts(0), WRAP_CONTRACT.parse().unwrap());
        contract.add_subscription(accounts(1), accounts(2), 0);
    }
}
