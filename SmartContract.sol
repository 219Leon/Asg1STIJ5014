// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ETH_HTLC {
    struct Swap {
        bytes32 hash;
        address payable sender;
        address payable receiver;
        uint256 amount;
        uint256 timelock;
        bool claimed;
        bool refunded;
    }

    mapping(bytes32 => Swap) public swaps;

    event SwapCreated(bytes32 indexed swapId, address sender, address receiver, uint256 amount, uint256 timelock);
    event SwapClaimed(bytes32 indexed swapId, bytes32 preimage);
    event SwapRefunded(bytes32 indexed swapId);

    function createSwap(bytes32 swapId, bytes32 hash, address payable receiver, uint256 timelock) external payable {
        require(msg.value > 0, "Amount must be positive");
        require(swaps[swapId].sender == address(0), "Swap ID already exists");

        swaps[swapId] = Swap({
            hash: hash,
            sender: payable(msg.sender),
            receiver: receiver,
            amount: msg.value,
            timelock: block.timestamp + timelock,
            claimed: false,
            refunded: false
        });

        emit SwapCreated(swapId, msg.sender, receiver, msg.value, timelock);
    }

    function claim(bytes32 swapId, bytes32 preimage) external {
        Swap storage swap = swaps[swapId];
        require(swap.hash == sha256(abi.encodePacked(preimage)), "Invalid preimage");
        require(!swap.claimed, "Already claimed");
        require(msg.sender == swap.receiver, "Not receiver");
        require(block.timestamp <= swap.timelock, "Timelock expired");

        swap.claimed = true;
        swap.receiver.transfer(swap.amount);

        emit SwapClaimed(swapId, preimage);
    }

    function refund(bytes32 swapId) external {
        Swap storage swap = swaps[swapId];
        require(block.timestamp > swap.timelock, "Timelock not expired");
        require(!swap.refunded, "Already refunded");
        require(msg.sender == swap.sender, "Not sender");

        swap.refunded = true;
        swap.sender.transfer(swap.amount);

        emit SwapRefunded(swapId);
    }
}