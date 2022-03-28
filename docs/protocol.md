# Protocol
- [ ] Add subscription
	- [x] Source, destination and rate
	- [ ] Confirm reserve is sufficient ::assert::
	- [x] All subscriptions are outgoing
	- [x] Increment subscription index by one
	- [x] Insert subscription into subscription map
	- [x] Insert into `Outputs` source and append subscription index
	- [x] Insert into `Inputs` destination and append subscription index
	- [x] Add start time to subscription
	
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
	- [x] Settle balance by transfer of near to account

```rust
fn remove_subscription(
	&mut self, 
	subscription_id: SubscriptionIndex
);
```

- [x] Calculate balance with subscriptions
	- [x] Calculate total of inputs
	- [x] Calculate total of outputs
	- [x] Calculate net for balance
- [x] Update subscription
	- [x] Update flow only ::assert:: if the same
	- [x] Settle based on old flow

```rust
fn update_subscription(
	&mut self, 
	subscription_id: SubscriptionIndex,
	new_flow: YoctoPerSecond
);
```

- [x] Subscriptions by account
	- [x] `AccountId` ::assert::

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
  timestamp: u128,
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
