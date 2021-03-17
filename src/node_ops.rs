// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use bls::PublicKeySet;
#[cfg(feature = "simulated-payouts")]
use sn_data_types::Transfer;
use sn_data_types::{
    ActorHistory, Blob, BlobAddress, Credit, CreditAgreementProof, NodeRewardStage, PublicKey,
    ReplicaEvent, SectionElders, SignatureShare, SignedCredit, SignedTransfer, SignedTransferShare,
    Token, TransferAgreementProof, TransferValidated, WalletHistory,
};
use sn_messaging::{
    client::{BlobRead, BlobWrite, Message, NodeSystemCmd},
    Aggregation, DstLocation, EndUser, MessageId, SrcLocation,
};
use sn_routing::{Elders, NodeElderChange, Prefix};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Formatter},
};
use xor_name::XorName;

/// Internal messages are what is passed along
/// within a node, between the entry point and
/// exit point of remote messages.
/// In other words, when communication from another
/// participant at the network arrives, it is analysed
/// and interpreted into an internal message, that can
/// then be passed along to its proper processing module
/// at the node. At a node module, the result of such a call
/// is also an internal message.
/// Finally, an internal message might be destined for Messaging
/// module, by which it leaves the process boundary of this node
/// and is sent on the wire to some other destination(s) on the network.

/// Vec of NodeDuty
pub type NodeDuties = Vec<NodeDuty>;

/// Vec of NetworkDuty
pub type NetworkDuties = Vec<NetworkDuty>;

/// All duties carried out by
/// a node in the network.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum NetworkDuty {
    RunAsAdult(AdultDuty),
    RunAsElder(ElderDuty),
    RunAsNode(NodeDuty),
    NoOp,
}

// --------------- Node ---------------

/// Common duties run by all nodes.
#[allow(clippy::large_enum_variant)]
pub enum NodeDuty {
    GetNodeWalletKey {
        old_node_id: XorName,
        new_node_id: XorName,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    PayoutNodeRewards {
        id: PublicKey,
        node_id: XorName,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    PropagateTransfer {
        proof: CreditAgreementProof,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    RegisterSectionPayout {
        debit_agreement: TransferAgreementProof,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    SetNodeWallet {
        wallet_id: PublicKey,
        node_id: XorName,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    ReceivePayoutValidation {
        validation: TransferValidated,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    GetTransferReplicaEvents {
        msg_id: MessageId,
        origin: SrcLocation,
    },
    ValidateSectionPayout {
        signed_transfer: SignedTransferShare,
        msg_id: MessageId,
        origin: SrcLocation,
    },
    ReadChunk {
        read: BlobRead,
        msg_id: MessageId,
        origin: EndUser,
    },
    WriteChunk {
        write: BlobWrite,
        msg_id: MessageId,
        origin: EndUser,
    },
    ContinueWalletChurn {
        replicas: SectionElders,
        msg_id: MessageId,
        origin: SrcLocation,
    },

    /// Get section elders.
    GetSectionElders {
        msg_id: MessageId,
        origin: SrcLocation,
    },
    /// On being promoted, an Adult node becomes an Elder.
    BeginFormingGenesisSection,
    /// Bootstrap of genesis section actor.
    ReceiveGenesisProposal {
        /// The genesis credit.
        credit: Credit,
        /// An individual elder's sig over the credit.
        sig: SignatureShare,
    },
    /// Bootstrap of genesis section actor.
    ReceiveGenesisAccumulation {
        /// The genesis credit.
        signed_credit: SignedCredit,
        /// An individual elder's sig over the credit.
        sig: SignatureShare,
    },
    /// Elder changes means the section public key
    /// changes as well, which leads to necessary updates
    /// of various places using the multisig of the section.
    UpdateElderInfo {
        /// The prefix of our section.
        prefix: Prefix,
        /// The BLS public key of our section.
        key: PublicKey,
        /// The set of elders of our section.
        elders: BTreeSet<XorName>,
        /// Sibling section PK if a split is underway
        sibling_key: Option<PublicKey>,
    },
    /// Finishes the multi-step process
    /// of transitioning to a new elder constellation.
    CompleteElderChange {
        /// The previous section key.
        previous_key: PublicKey,
        /// The new section key.
        new_key: PublicKey,
    },
    ChurnMembers {
        /// The Elders of our section.
        elders: Elders,
        /// The Elders of the sibling section, if this event is fired during a split.
        /// Otherwise `None`.
        sibling_elders: Option<Elders>,
    },
    /// When promoted, node levels up
    LevelUp,
    /// When demoted, node levels down
    LevelDown,
    /// Initiates the node with state from peers.
    ContinueLevelUp {
        /// The registered wallet keys for nodes earning rewards
        node_rewards: BTreeMap<XorName, NodeRewardStage>,
        /// The wallets of users on the network.
        user_wallets: BTreeMap<PublicKey, ActorHistory>,
    },
    /// Initiates the section wallet.
    CompleteLevelUp(WalletHistory),
    ProcessNewMember(XorName),
    /// As members are lost for various reasons
    /// there are certain things nodes need
    /// to do, to update for that.
    ProcessLostMember {
        name: XorName,
        age: u8,
    },
    ProcessRelocatedMember {
        /// The id of the node at the previous section.
        old_node_id: XorName,
        /// The id of the node at its new section (i.e. this one).
        new_node_id: XorName,
        // The age of the node (among things determines if it is eligible for rewards yet).
        age: u8,
    },
    /// Storage reaching max capacity.
    ReachingMaxCapacity,
    /// Increment count of full nodes in the network
    IncrementFullNodeCount {
        /// Node ID of node that reached max capacity.
        node_id: PublicKey,
    },
    SwitchNodeJoin(bool),
    /// Send a message to the specified dst.
    Send(OutgoingMsg),
    /// Send the same request to each individual node.
    SendToNodes {
        targets: BTreeSet<XorName>,
        msg: Message,
    },
    /// Process read of data
    ProcessRead {
        query: sn_messaging::client::DataQuery,
        id: MessageId,
        origin: EndUser,
    },
    /// Process write of data
    ProcessWrite {
        cmd: sn_messaging::client::DataCmd,
        id: MessageId,
        origin: EndUser,
    },
    /// Process Payment for a DataCmd
    ProcessDataPayment {
        msg: Message,
        origin: EndUser,
    },
    /// Process replication of a chunk on `MemberLeft`
    /// This is run at the node which is the new holder
    /// of a chunk
    ReplicateChunk {
        address: BlobAddress,
        current_holders: BTreeSet<XorName>,
        id: MessageId,
    },
    /// Process a GetChunk operation
    /// and send it back to to the requesting node
    /// for replication
    GetChunkForReplication {
        address: BlobAddress,
        new_holder: XorName,
        id: MessageId,
    },
    /// Store a chunk that is a result of data replication
    /// on `MemberLeft`
    StoreChunkForReplication {
        data: Blob,
        correlation_id: MessageId,
    },
    NoOp,
}

impl From<NodeDuty> for NodeDuties {
    fn from(duty: NodeDuty) -> Self {
        if matches!(duty, NodeDuty::NoOp) {
            vec![]
        } else {
            vec![duty]
        }
    }
}

impl Debug for NodeDuty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GetNodeWalletKey { .. } => write!(f, "GetNodeWalletKey"),
            Self::PayoutNodeRewards { .. } => write!(f, "PayoutNodeRewards"),
            Self::PropagateTransfer { .. } => write!(f, "PropagateTransfer"),
            Self::RegisterSectionPayout { .. } => write!(f, "RegisterSectionPayout"),
            Self::SetNodeWallet { .. } => write!(f, "SetNodeWallet"),
            Self::ReceivePayoutValidation { .. } => write!(f, "ReceivePayoutValidation"),
            Self::GetTransferReplicaEvents { .. } => write!(f, "GetTransferReplicaEvents"),
            Self::ValidateSectionPayout { .. } => write!(f, "ValidateSectionPayout"),
            Self::ReadChunk { .. } => write!(f, "ReadChunk"),
            Self::WriteChunk { .. } => write!(f, "WriteChunk"),
            Self::ContinueWalletChurn { .. } => write!(f, "ContinueWalletChurn"),
            // ------
            Self::LevelUp => write!(f, "LevelUp"),
            Self::LevelDown => write!(f, "LevelDown"),
            Self::ContinueLevelUp { .. } => write!(f, "ContinueLevelUp"),
            Self::CompleteLevelUp { .. } => write!(f, "CompleteLevelUp"),
            Self::ChurnMembers { .. } => write!(f, "ChurnMembers"),
            Self::GetSectionElders { .. } => write!(f, "GetSectionElders"),
            Self::ReceiveGenesisProposal { .. } => write!(f, "ReceiveGenesisProposal"),
            Self::ReceiveGenesisAccumulation { .. } => write!(f, "ReceiveGenesisAccumulation"),
            Self::BeginFormingGenesisSection => write!(f, "BeginFormingGenesisSection"),

            Self::NoOp => write!(f, "No op."),
            Self::ReachingMaxCapacity => write!(f, "ReachingMaxCapacity"),
            Self::UpdateElderInfo { .. } => write!(f, "UpdateElderInfo"),
            Self::CompleteElderChange { .. } => write!(f, "CompleteElderChange"),
            Self::ProcessNewMember(_) => write!(f, "ProcessNewMember"),
            Self::ProcessLostMember { .. } => write!(f, "ProcessLostMember"),
            Self::ProcessRelocatedMember { .. } => write!(f, "ProcessRelocatedMember"),
            Self::IncrementFullNodeCount { .. } => write!(f, "IncrementFullNodeCount"),
            Self::SwitchNodeJoin(_) => write!(f, "SwitchNodeJoin"),
            Self::Send(msg) => write!(f, "Send [ msg: {:?} ]", msg),
            Self::SendToNodes { targets, msg } => {
                write!(f, "SendToNodes [ targets: {:?}, msg: {:?} ]", targets, msg)
            }
            Self::ProcessRead { .. } => write!(f, "ProcessRead"),
            Self::ProcessWrite { .. } => write!(f, "ProcessWrite"),
            Self::ProcessDataPayment { .. } => write!(f, "ProcessDataPayment"),
            Self::ReplicateChunk { .. } => write!(f, "ReplicateChunk"),
            Self::GetChunkForReplication { .. } => write!(f, "GetChunkForReplication"),
            Self::StoreChunkForReplication { .. } => write!(f, "StoreChunkForReplication"),
        }
    }
}

// --------------- Messaging ---------------

#[derive(Debug, Clone)]
pub struct OutgoingMsg {
    pub msg: Message,
    pub dst: DstLocation,
    pub section_source: bool,
    pub aggregation: Aggregation,
}

impl OutgoingMsg {
    pub fn id(&self) -> MessageId {
        self.msg.id()
    }
}

/// This duty is at the border of infrastructural
/// and domain duties. Messaging is such a fundamental
/// part of the system, that it can be considered domain.
#[allow(clippy::large_enum_variant)]
pub enum NodeMessagingDuty {
    // No operation
    NoOp,
}

impl Debug for NodeMessagingDuty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOp => write!(f, "No op."),
        }
    }
}

// --------------- Adult ---------------

/// Duties only run as an Adult.
#[derive(Debug)]
pub enum AdultDuty {
    /// The main duty of an Adult is
    /// storage and retrieval of data chunks.
    RunAsChunkStore(ChunkStoreDuty),
    RunAsChunkReplication(ChunkReplicationDuty),
    NoOp,
}

impl From<AdultDuty> for NetworkDuties {
    fn from(duty: AdultDuty) -> Self {
        use NetworkDuty::*;
        if matches!(duty, AdultDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsAdult(duty)]
        }
    }
}

impl From<AdultDuty> for NetworkDuty {
    fn from(duty: AdultDuty) -> Self {
        use NetworkDuty::*;
        RunAsAdult(duty)
    }
}

// --------------- Elder ---------------

/// Duties only run as an Elder.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ElderDuty {
    ProcessNewMember(XorName),
    /// As members are lost for various reasons
    /// there are certain things the Elders need
    /// to do, to update for that.
    ProcessLostMember {
        name: XorName,
        age: u8,
    },
    ProcessRelocatedMember {
        /// The id of the node at the previous section.
        old_node_id: XorName,
        /// The id of the node at its new section (i.e. this one).
        new_node_id: XorName,
        // The age of the node (among things determines if it is eligible for rewards yet).
        age: u8,
    },
    /// A key section interfaces with clients.
    RunAsKeySection(KeySectionDuty),
    /// A data section receives requests relayed
    /// via key sections.
    RunAsDataSection(DataSectionDuty),
    NoOp,
    /// Increase number of Full Nodes in the network
    StorageFull {
        /// Node ID of node that reached max capacity.
        node_id: PublicKey,
    },
    SwitchNodeJoin(bool),
}

impl From<ElderDuty> for NetworkDuties {
    fn from(duty: ElderDuty) -> Self {
        use NetworkDuty::*;
        if matches!(duty, ElderDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsElder(duty)]
        }
    }
}

impl From<ElderDuty> for NetworkDuty {
    fn from(duty: ElderDuty) -> Self {
        use NetworkDuty::*;
        RunAsElder(duty)
    }
}

// --------------- KeySection ---------------

/// Duties only run as a Key section.
#[derive(Debug)]
pub enum KeySectionDuty {
    /// Transfers of tokens between keys, hence also payment for data writes.
    RunAsTransfers(TransferDuty),
    NoOp,
}

impl From<KeySectionDuty> for NetworkDuties {
    fn from(duty: KeySectionDuty) -> Self {
        use ElderDuty::*;
        use NetworkDuty::*;
        if matches!(duty, KeySectionDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsElder(RunAsKeySection(duty))]
        }
    }
}

// --------------- DataSection ---------------

/// Duties only run as a Data section.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DataSectionDuty {
    /// Metadata is the info about
    /// data types structures, ownership
    /// and permissions. This is distinct
    /// from the actual data, that is in chunks.
    /// NB: Full separation between metadata and chunks is not yet implemented.
    RunAsMetadata(MetadataDuty),
    /// Dealing out rewards for contributing to
    /// the network by storing metadata / data, and
    /// carrying out operations on those.
    RunAsRewards(RewardDuty),
    NoOp,
}

// --------------- Metadata ---------------

/// Reading and writing data.
/// The reads/writes potentially concerns
/// metadata only, but could include
/// chunks, and are then relayed to
/// Adults (i.e. chunk holders).
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum MetadataDuty {
    /// Reads.
    ProcessRead {
        query: sn_messaging::client::DataQuery,
        id: MessageId,
        origin: EndUser,
    },
    /// Writes.
    ProcessWrite {
        cmd: sn_messaging::client::DataCmd,
        id: MessageId,
        origin: EndUser,
    },
    NoOp,
}

impl From<MetadataDuty> for NetworkDuties {
    fn from(duty: MetadataDuty) -> Self {
        use DataSectionDuty::*;
        use ElderDuty::*;
        use NetworkDuty::*;
        if matches!(duty, MetadataDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsElder(RunAsDataSection(RunAsMetadata(duty)))]
        }
    }
}

// --------------- Chunks ---------------

/// Chunk storage and retrieval is done at Adults.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ChunkStoreDuty {
    /// Reads.
    ReadChunk {
        read: BlobRead,
        id: MessageId,
        origin: EndUser,
    },
    /// Writes.
    WriteChunk {
        write: BlobWrite,
        id: MessageId,
        origin: EndUser,
    },
    NoOp,
}

/// Chunk storage and retrieval is done at Adults.
#[derive(Debug)]
pub enum ChunkReplicationDuty {
    ///
    ProcessCmd {
        cmd: ChunkReplicationCmd,
        ///
        msg_id: MessageId,
        // ///
        origin: SrcLocation,
    },
    ///
    ProcessQuery {
        query: ChunkReplicationQuery,
        ///
        msg_id: MessageId,
        // ///
        origin: SrcLocation,
    },
    NoOp,
}

/// Queries for chunk to replicate
#[derive(Debug)]
pub enum ChunkReplicationQuery {
    ///
    GetChunk(BlobAddress),
}

/// Cmds carried out on Adults.
#[derive(Debug)]
#[allow(clippy::clippy::large_enum_variant)]
pub enum ChunkReplicationCmd {
    /// An imperament to retrieve
    /// a chunk from current holders, in order
    /// to replicate it locally.
    ReplicateChunk {
        ///
        current_holders: BTreeSet<XorName>,
        ///
        address: BlobAddress,
        // ///
        // section_authority: MsgSender,
    },
    StoreReplicatedBlob(Blob),
}

// --------------- Rewards ---------------

/// Nodes participating in the system are
/// rewarded for their work.
/// Elders are responsible for the duties of
/// keeping track of rewards, and issuing
/// payouts from the section account.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum RewardDuty {
    ///
    ProcessQuery {
        query: RewardQuery,
        ///
        msg_id: MessageId,
        ///
        origin: SrcLocation,
    },
    ///
    ProcessCmd {
        cmd: RewardCmd,
        ///
        msg_id: MessageId,
        ///
        origin: SrcLocation,
    },
    NoOp,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RewardCmd {
    /// Initiates a new SectionActor with the
    /// state of existing Replicas in the group.
    SynchHistory(WalletHistory),
    /// Completes transition to a new SectionActor, i.e. new wallet,
    /// and sets it up with its replicas' public key set.
    ContinueWalletChurn(PublicKeySet),
    /// With the node id.
    AddNewNode(XorName),
    /// Set the account for a node.
    SetNodeWallet {
        /// The node which accumulated the rewards.
        node_id: XorName,
        /// The account to which the accumulated
        /// rewards should be paid out.
        wallet_id: PublicKey,
    },
    /// We add relocated nodes to our rewards
    /// system, so that they can participate
    /// in the farming rewards.
    AddRelocatingNode {
        /// The id of the node at the previous section.
        old_node_id: XorName,
        /// The id of the node at its new section (i.e. this one).
        new_node_id: XorName,
        // The age of the node, determines if it is eligible for rewards yet.
        age: u8,
    },
    /// When a node has been relocated to our section
    /// we receive the account id from the other section.
    ActivateNodeRewards {
        /// The account to which the accumulated
        /// rewards should be paid out.
        id: PublicKey,
        /// The node which accumulated the rewards.
        node_id: XorName,
    },
    /// When a node has left for some reason,
    /// we deactivate it.
    DeactivateNode(XorName),
    /// The distributed Actor of a section,
    /// receives and accumulates the validated
    /// reward payout from its Replicas,
    ReceivePayoutValidation(TransferValidated),
}

/// payouts from the section account.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RewardQuery {
    /// When a node is relocated from us, the other
    /// section will query for the node wallet id.
    GetNodeWalletId {
        /// The id of the node at the previous section.
        old_node_id: XorName,
        /// The id of the node at its new section (i.e. this one).
        new_node_id: XorName,
    },
    // /// When a new Section Actor share joins,
    // /// it queries the other shares for the section wallet history.
    // GetSectionWalletHistory,
}

impl From<RewardDuty> for NetworkDuties {
    fn from(duty: RewardDuty) -> Self {
        use DataSectionDuty::*;
        use ElderDuty::*;
        use NetworkDuty::*;
        if matches!(duty, RewardDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsElder(RunAsDataSection(RunAsRewards(duty)))]
        }
    }
}

impl From<RewardDuty> for NetworkDuty {
    fn from(duty: RewardDuty) -> Self {
        use DataSectionDuty::*;
        use ElderDuty::*;
        use NetworkDuty::*;
        RunAsElder(RunAsDataSection(RunAsRewards(duty)))
    }
}

// --------------- Transfers ---------------

/// Transfers of tokens on the network
/// and querying of balances and history.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum TransferDuty {
    ///
    ProcessQuery {
        query: TransferQuery,
        ///
        msg_id: MessageId,
        ///
        origin: SrcLocation,
    },
    ///
    ProcessCmd {
        cmd: TransferCmd,
        ///
        msg_id: MessageId,
        ///
        origin: SrcLocation,
    },
    NoOp,
}

impl From<TransferDuty> for NetworkDuties {
    fn from(duty: TransferDuty) -> Self {
        use ElderDuty::*;
        use KeySectionDuty::*;
        use NetworkDuty::*;
        if matches!(duty, TransferDuty::NoOp) {
            vec![]
        } else {
            vec![RunAsElder(RunAsKeySection(RunAsTransfers(duty)))]
        }
    }
}

/// Queries for information on accounts,
/// handled by AT2 Replicas.
#[derive(Debug)]
pub enum TransferQuery {
    /// Get the PublicKeySet for replicas of a given PK
    GetReplicaKeys(PublicKey),
    /// Get key balance.
    GetBalance(PublicKey),
    /// Get key transfers since specified version.
    GetHistory {
        /// The wallet key.
        at: PublicKey,
        /// The last version of transfers we know of.
        since_version: usize,
    },
    GetReplicaEvents,
    /// Get the latest cost for writing given number of bytes to network.
    GetStoreCost {
        /// The requester's key.
        requester: PublicKey,
        /// Number of bytes to write.
        bytes: u64,
    },
}

/// Cmds carried out on AT2 Replicas.
#[derive(Debug)]
#[allow(clippy::clippy::large_enum_variant)]
pub enum TransferCmd {
    /// Initiates a new Replica with the
    /// state of existing Replicas in the group.
    InitiateReplica(Vec<ReplicaEvent>),
    ProcessPayment(Message),
    #[cfg(feature = "simulated-payouts")]
    /// Cmd to simulate a farming payout
    SimulatePayout(Transfer),
    /// The cmd to validate a transfer.
    ValidateTransfer(SignedTransfer),
    /// The cmd to register the consensused transfer.
    RegisterTransfer(TransferAgreementProof),
    /// As a transfer has been propagated to the
    /// crediting section, it is applied there.
    PropagateTransfer(CreditAgreementProof),
    /// The validation of a section transfer.
    ValidateSectionPayout(SignedTransferShare),
    /// The registration of a section transfer.
    RegisterSectionPayout(TransferAgreementProof),
}

impl From<sn_messaging::client::TransferCmd> for TransferCmd {
    fn from(cmd: sn_messaging::client::TransferCmd) -> Self {
        match cmd {
            #[cfg(feature = "simulated-payouts")]
            sn_messaging::client::TransferCmd::SimulatePayout(transfer) => {
                Self::SimulatePayout(transfer)
            }
            sn_messaging::client::TransferCmd::ValidateTransfer(signed_transfer) => {
                Self::ValidateTransfer(signed_transfer)
            }
            sn_messaging::client::TransferCmd::RegisterTransfer(transfer_agreement) => {
                Self::RegisterTransfer(transfer_agreement)
            }
        }
    }
}

impl From<sn_messaging::client::TransferQuery> for TransferQuery {
    fn from(cmd: sn_messaging::client::TransferQuery) -> Self {
        match cmd {
            sn_messaging::client::TransferQuery::GetReplicaKeys(transfer) => {
                Self::GetReplicaKeys(transfer)
            }
            sn_messaging::client::TransferQuery::GetBalance(public_key) => {
                Self::GetBalance(public_key)
            }
            sn_messaging::client::TransferQuery::GetHistory { at, since_version } => {
                Self::GetHistory { at, since_version }
            }
            sn_messaging::client::TransferQuery::GetStoreCost { requester, bytes } => {
                Self::GetStoreCost { requester, bytes }
            }
        }
    }
}