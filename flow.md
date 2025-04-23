Fetch quote

Use this endpoint to fetch pricing for a given OrderPair and amount. The response will return an array of objects, where each object contains a strategy_id as the key and the corresponding amount as the value. The amount will be in the smallest decimal unit of the source asset or destination asset, depending on the exact_out flag. Select a strategy from the response and save the strategy_id, as it must be passed in the next steps.

curl -X 'GET' \
 'https://api.garden.finance/testnet/prices/price?order_pair=<order_pair>&amount=<amount>&exact_out=<true/false>' \
 -H 'accept: application/json'

Parameters:

    order_pair: String representation of OrderPair.
    amount: The amount should be in the smallest unit of the source asset or destination asset depending on the exact_out flag.
    exact_out: Indicates whether the quote should be fetched for an exact output amount. If set to true, the quote will calculate the required input amount to achieve the specified output. If set to false, the quote will calculate the expected output for a given input amount.

Create order

Creating an order involves two steps: attesting the quote and then creating the order.
Attest quote

First, you need to attest the quote by submitting the strategy_id obtained from the previous step along with the complete order details. This step verifies the quote and all other details of the order, confirming the pricing. In response, you'll receive the same object with added signature, deadline, and asset price fields inside additional_data, which you will use in the next step to create the order. The order should be created and initiated within the deadline to ensure the quote remains valid.

curl -X 'POST' \
 'https://api.garden.finance/testnet/prices/quote/attested' \
 -H 'accept: application/json' \
 -H 'Content-Type: application/json' \
 -d '{
"source_chain": "<source_chain>",
"destination_chain": "<destination_chain>",
"source_asset": "<source_asset>",
"destination_asset": "<destination_asset>",
"initiator_source_address": "<initiator_source_address>",
"initiator_destination_address": "<initiator_destination_address>",
"source_amount": "<source_amount>",
"destination_amount": "<destination_amount>",
"fee": "<fee>",
"nonce": "<nonce>",
"min_destination_confirmations": "<min_destination_confirmations>",
"timelock": "<timelock>",
"secret_hash": "<secret_hash>",
"additional_data": {
"strategy_id": "<strategy_id>",
"bitcoin_optional_recipient": "<user_bitcoin_address>",
}
}'

Parameters:

    bitcoin_optional_recipient(optional): The user's Bitcoin address, to be provided if either the source or destination asset is Bitcoin.
    timelock: The timelock value should be provided in the source chain's block numbers, calculated based on the block time for a 24-hour period.
    nonce: The nonce is a unique identifier used to manage secrets. To generate a secret, we sign this nonce with the user's wallet. While the nonce can technically be any random string, in Garden we use it as the total number of orders placed so far, plus one. You can find the number of orders by checking the /user/count endpoint.

Create order

After attesting the quote, send the response from the attested quote to this endpoint to create the order. This will return the order ID, which you will use in the next step to initiate the order or retrieve the order details.

curl -X 'POST' \
 'https://api.garden.finance/testnet/orders/gasless/order' \
 -H 'accept: application/json' \
 -H 'Content-Type: application/json' \
 -H 'Authorization: Bearer <authorization_token>' \
 -d '{
...response_from_attest_quote
}'

The order is considered successfully created and matched if you receive a valid order object response from the getOrder endpoint.
Get order

Retrieve the order details using the order ID.

curl -X 'GET' \
 'https://api.garden.finance/testnet/orders/orders/id/matched/<order_id>' \
 -H 'accept: application/json'

Initiate order

For Bitcoin initiation, the user must send the exact amount of funds to the order.source_swap.swap_id address.

For EVM-based initiation, you can either directly interact with the contract to transfer the funds or use Garden's relay service to facilitate the transaction.

To initiate the order using the relay service, the user must sign a message following the EIP-712 standard. The message must include the following details:

    redeemer: order.source_swap.redeemer – The address of the party who will redeem.
    timelock: order.create_order.timelock
    amount: order.source_swap.amount – The amount to be swapped.
    secretHash: order.create_order.secret_hash (without 0x prefix) – The hash of the secret used in the swap.

curl -X 'POST' \
 'https://api.garden.finance/testnet/orders/gasless/initiate' \
 -H 'accept: application/json' \
 -H 'Content-Type: application/json' \
 -d '{
"order_id": "<order_id>",
"signature": "<signature>",
"perform_on": "Source"
}'
