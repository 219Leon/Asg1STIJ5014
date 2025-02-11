#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_io::hashing::sha2_256;
    use sp_std::vec::Vec;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: ReservableCurrency<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn swaps)]
    pub type Swaps<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Swap<T::AccountId, BlockNumberFor<T>, BalanceOf<T>>,
    >;

    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct Swap<AccountId, BlockNumber, Balance> {
        pub hash: [u8; 32],
        pub sender: AccountId,
        pub receiver: AccountId,
        pub amount: Balance,
        pub timelock: BlockNumber,
        pub claimed: bool,
        pub refunded: bool,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SwapCreated { swap_id: T::Hash, sender: T::AccountId, receiver: T::AccountId, amount: BalanceOf<T>, timelock: BlockNumberFor<T> },
        SwapClaimed { swap_id: T::Hash, preimage: Vec<u8> },
        SwapRefunded { swap_id: T::Hash },
    }

    #[pallet::error]
    pub enum Error<T> {
        SwapExists,
        InvalidPreimage,
        AlreadyClaimed,
        AlreadyRefunded,
        TimelockNotExpired,
        NotReceiver,
        NotSender,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn create_swap(
            origin: OriginFor<T>,
            swap_id: T::Hash,
            hash: [u8; 32],
            receiver: T::AccountId,
            timelock: BlockNumberFor<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(!Swaps::<T>::contains_key(&swap_id), Error::<T>::SwapExists);

            T::Currency::reserve(&sender, amount)?;

            let swap = Swap {
                hash,
                sender: sender.clone(),
                receiver: receiver.clone(),
                amount,
                timelock: frame_system::Pallet::<T>::block_number() + timelock,
                claimed: false,
                refunded: false,
            };

            Swaps::<T>::insert(swap_id, swap);

            Self::deposit_event(Event::SwapCreated { swap_id, sender, receiver, amount, timelock });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn claim(
            origin: OriginFor<T>,
            swap_id: T::Hash,
            preimage: Vec<u8>,
        ) -> DispatchResult {
            let receiver = ensure_signed(origin)?;
            let mut swap = Swaps::<T>::get(&swap_id).ok_or(Error::<T>::InvalidSwapId)?;

            let hash = sha2_256(&preimage);
            ensure!(swap.hash == hash, Error::<T>::InvalidPreimage);
            ensure!(!swap.claimed, Error::<T>::AlreadyClaimed);
            ensure!(receiver == swap.receiver, Error::<T>::NotReceiver);
            ensure!(frame_system::Pallet::<T>::block_number() <= swap.timelock, Error::<T>::TimelockExpired);

            T::Currency::unreserve(&swap.sender, swap.amount);
            T::Currency::transfer(&swap.sender, &receiver, swap.amount, ExistenceRequirement::KeepAlive)?;

            swap.claimed = true;
            Swaps::<T>::insert(swap_id, swap);

            Self::deposit_event(Event::SwapClaimed { swap_id, preimage });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn refund(
            origin: OriginFor<T>,
            swap_id: T::Hash,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let mut swap = Swaps::<T>::get(&swap_id).ok_or(Error::<T>::InvalidSwapId)?;

            ensure!(frame_system::Pallet::<T>::block_number() > swap.timelock, Error::<T>::TimelockNotExpired);
            ensure!(!swap.refunded, Error::<T>::AlreadyRefunded);
            ensure!(sender == swap.sender, Error::<T>::NotSender);

            T::Currency::unreserve(&swap.sender, swap.amount);

            swap.refunded = true;
            Swaps::<T>::insert(swap_id, swap);

            Self::deposit_event(Event::SwapRefunded { swap_id });
            Ok(())
        }
    }
}