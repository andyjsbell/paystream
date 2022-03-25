# Protocol
- [ ] Add subscription
	- [x] Source, destination and rate
	- [ ] Confirm reserve is sufficient ::assert::
	- [ ] All subscriptions are outgoing
	- [x] Increment subscription index by one
	- [x] Insert subscription into subscription map
	- [x] Insert into `Outputs` source and append subscription index
	- [x] Insert into `Inputs` destination and append subscription index
	- [ ] Add start time to subscription
	
```rust
fn add_subscription(
	&mut self,
  start: u64, // time stamp seconds since epoch
	source: AccountId, 
	destination: AccountId, 
	flow: YoctoPerSecond
);
```

- [ ] Remove subscription
	- [x] Subscription index ::assert::
	- [x] Available to source and destination ::assert:: 
	- [ ] Settle balance by transfer of near to account

```rust
fn remove_subscription(
	&mut self, 
	subscription_id: SubscriptionIndex
);
```

- [ ] Calculate balance with subscriptions
	- [ ] Calculate total of inputs
	- [ ] Calculate total of outputs
	- [ ] Calculate net for balance
- [ ] Update subscription
	- [ ] Update flow only ::assert:: if the same
	- [ ] Settle based on old flow

```rust
fn update_subscription(
	&mut self, 
	subscription_id: SubscriptionIndex,
	new_flow: YoctoPerSecond
);
```

- [ ] Subscriptions by account
	- [ ] `AccountId` ::assert::

```rust
fn subscriptions(
	&self
) -> Vec<SubscriptionIndex>;
```

- [ ] Add Reporter
	- [ ] `AccountId` and stake above minimum ::assert::
- [ ] Remove Reporter
	- [ ] `AccountId` is **Reporter** ::assert::
	- [ ] Remove account 
	- [ ] Return stake
- [ ] Claim Reporter
	- [ ] `AccountId` ::assert::
	- [ ] Above minimum after claim ::assert::
	- [ ] Transfer to **Reporter**
- [ ] Report source
	- [ ] `AccountId` of **Reporter** ::assert::
	- [ ] `AccountId` of source ::assert:: that it is beneath level
	- [ ] Fixed percentage goes to **Reporter**
	- [ ] Fixed percentage goes to **Treasury**
- [x] Set treasury account
	- [x] Set by **Owner** ::assert::
- [x] Pay from treasury
	- [x] Paid as token by key holder of treasury account
	- [x] -**Treasury** ::assert::-
	- [x] -Destination account-
	
## Data structures
### Owner
```rust
owner: AccountId
```

### Treasury
```rust
treasury: AccountId
```

### Balances
```rust
LookupMap<AccountId, Balance>
```

### Subscriptions
```rust
let subscriptionIndex: SubscriptionIndex = 0;
type YoctoPerSecond = u128;
Subscription {
  source: SourceAccountId,
  destination: DestinationAccountId,
	rate: YoctoPerSecond,
}
LookupMap<SubscriptionIndex, Subscription> 
```

### Outputs
```rust
LookupMap<SourceAccountId, Vec<SubscriptionIndex>>
```

### Inputs
```rust
LookupMap<DestinationAccountId, Vec<SubscriptionId>>
```