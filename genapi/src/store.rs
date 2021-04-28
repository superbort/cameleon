use std::{
    collections::{hash_map, HashMap},
    convert::TryFrom,
};

use string_interner::{StringInterner, Symbol};

use super::{
    interface::{
        IBooleanKind, ICategoryKind, ICommandKind, IEnumerationKind, IFloatKind, IIntegerKind,
        IPortKind, IRegisterKind, ISelectorKind, IStringKind,
    },
    node_base::NodeBase,
    BooleanNode, CategoryNode, CommandNode, ConverterNode, EnumerationNode, FloatNode,
    FloatRegNode, GenApiError, GenApiResult, IntConverterNode, IntRegNode, IntSwissKnifeNode,
    IntegerNode, MaskedIntRegNode, Node, PortNode, RegisterNode, StringNode, StringRegNode,
    SwissKnifeNode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl Symbol for NodeId {
    fn try_from_usize(index: usize) -> Option<Self> {
        if ((u32::MAX - 1) as usize) < index {
            None
        } else {
            #[allow(clippy::cast_possible_truncation)]
            Some(Self(index as u32))
        }
    }

    fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl NodeId {
    pub fn as_iinteger_kind<'a>(self, store: &'a impl NodeStore) -> Option<IIntegerKind<'a>> {
        IIntegerKind::maybe_from(self, store)
    }

    pub fn expect_iinteger_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<IIntegerKind<'a>> {
        self.as_iinteger_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `IInteger`".into(),
        ))
    }

    pub fn as_ifloat_kind<'a>(self, store: &'a impl NodeStore) -> Option<IFloatKind<'a>> {
        IFloatKind::maybe_from(self, store)
    }

    pub fn expect_ifloat_kind<'a>(self, store: &'a impl NodeStore) -> GenApiResult<IFloatKind<'a>> {
        self.as_ifloat_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `IFloat`".into(),
        ))
    }

    pub fn as_istring_kind<'a>(self, store: &'a impl NodeStore) -> Option<IStringKind<'a>> {
        IStringKind::maybe_from(self, store)
    }

    pub fn expect_istring_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<IStringKind<'a>> {
        self.as_istring_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `IString`".into(),
        ))
    }

    pub fn as_icommand_kind<'a>(self, store: &'a impl NodeStore) -> Option<ICommandKind<'a>> {
        ICommandKind::maybe_from(self, store)
    }

    pub fn expect_icommand_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<ICommandKind<'a>> {
        self.as_icommand_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `ICommand`".into(),
        ))
    }

    pub fn as_ienuemration_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> Option<IEnumerationKind<'a>> {
        IEnumerationKind::maybe_from(self, store)
    }

    pub fn expect_ienumeration_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<IEnumerationKind<'a>> {
        self.as_ienuemration_kind(store)
            .ok_or(GenApiError::InvalidNode(
                "the node doesn't implement `IEnumeration`".into(),
            ))
    }

    pub fn as_iboolean_kind<'a>(self, store: &'a impl NodeStore) -> Option<IBooleanKind<'a>> {
        IBooleanKind::maybe_from(self, store)
    }

    pub fn expect_iboolean_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<IBooleanKind<'a>> {
        self.as_iboolean_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `IBoolean`".into(),
        ))
    }

    pub fn as_iregister_kind<'a>(self, store: &'a impl NodeStore) -> Option<IRegisterKind<'a>> {
        IRegisterKind::maybe_from(self, store)
    }

    pub fn expect_iregister_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<IRegisterKind<'a>> {
        self.as_iregister_kind(store)
            .ok_or(GenApiError::InvalidNode(
                "the node doesn't implement `IRegister`".into(),
            ))
    }

    pub fn as_icategory_kind<'a>(self, store: &'a impl NodeStore) -> Option<ICategoryKind<'a>> {
        ICategoryKind::maybe_from(self, store)
    }

    pub fn expect_icategory_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<ICategoryKind<'a>> {
        self.as_icategory_kind(store)
            .ok_or(GenApiError::InvalidNode(
                "the node doesn't implement `ICategory`".into(),
            ))
    }

    pub fn as_iport_kind<'a>(self, store: &'a impl NodeStore) -> Option<IPortKind<'a>> {
        IPortKind::maybe_from(self, store)
    }

    pub fn expect_iport_kind<'a>(self, store: &'a impl NodeStore) -> GenApiResult<IPortKind<'a>> {
        self.as_iport_kind(store).ok_or(GenApiError::InvalidNode(
            "the node doesn't implement `IPort`".into(),
        ))
    }

    pub fn as_iselector_kind<'a>(self, store: &'a impl NodeStore) -> Option<ISelectorKind<'a>> {
        ISelectorKind::maybe_from(self, store)
    }

    pub fn expect_iselector_kind<'a>(
        self,
        store: &'a impl NodeStore,
    ) -> GenApiResult<ISelectorKind<'a>> {
        self.as_iselector_kind(store)
            .ok_or(GenApiError::InvalidNode(
                "the node doesn't implement `ISelector`".into(),
            ))
    }
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Node(Box<Node>),
    Category(Box<CategoryNode>),
    Integer(Box<IntegerNode>),
    IntReg(Box<IntRegNode>),
    MaskedIntReg(Box<MaskedIntRegNode>),
    Boolean(Box<BooleanNode>),
    Command(Box<CommandNode>),
    Enumeration(Box<EnumerationNode>),
    Float(Box<FloatNode>),
    FloatReg(Box<FloatRegNode>),
    String(Box<StringNode>),
    StringReg(Box<StringRegNode>),
    Register(Box<RegisterNode>),
    Converter(Box<ConverterNode>),
    IntConverter(Box<IntConverterNode>),
    SwissKnife(Box<SwissKnifeNode>),
    IntSwissKnife(Box<IntSwissKnifeNode>),
    Port(Box<PortNode>),

    // TODO: Implement DCAM specific ndoes.
    ConfRom(()),
    TextDesc(()),
    IntKey(()),
    AdvFeatureLock(()),
    SmartFeature(()),
}

impl NodeData {
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn node_base(&self) -> NodeBase<'_> {
        match self {
            Self::Node(node) => node.node_base(),
            Self::Category(node) => node.node_base(),
            Self::Integer(node) => node.node_base(),
            Self::IntReg(node) => node.node_base(),
            Self::MaskedIntReg(node) => node.node_base(),
            Self::Boolean(node) => node.node_base(),
            Self::Command(node) => node.node_base(),
            Self::Enumeration(node) => node.node_base(),
            Self::Float(node) => node.node_base(),
            Self::FloatReg(node) => node.node_base(),
            Self::String(node) => node.node_base(),
            Self::StringReg(node) => node.node_base(),
            Self::Register(node) => node.node_base(),
            Self::Converter(node) => node.node_base(),
            Self::IntConverter(node) => node.node_base(),
            Self::SwissKnife(node) => node.node_base(),
            Self::IntSwissKnife(node) => node.node_base(),
            Self::Port(node) => node.node_base(),
            _ => todo!(),
        }
    }
}

pub trait NodeStore {
    fn id_by_name<T: AsRef<str>>(&mut self, s: T) -> NodeId;

    fn node_opt(&self, id: NodeId) -> Option<&NodeData>;

    fn store_node(&mut self, id: NodeId, data: NodeData);

    fn visit_nodes(&self, f: impl FnMut(&NodeData));

    fn node(&self, id: NodeId) -> &NodeData {
        self.node_opt(id).unwrap()
    }
}

impl<T> NodeStore for &mut T
where
    T: NodeStore,
{
    fn id_by_name<U: AsRef<str>>(&mut self, s: U) -> NodeId {
        (*self).id_by_name(s)
    }

    fn node_opt(&self, id: NodeId) -> Option<&NodeData> {
        (**self).node_opt(id)
    }

    fn store_node(&mut self, id: NodeId, data: NodeData) {
        (*self).store_node(id, data)
    }

    fn visit_nodes(&self, f: impl FnMut(&NodeData)) {
        (**self).visit_nodes(f);
    }
}

#[derive(Debug)]
pub struct DefaultNodeStore {
    interner: StringInterner<NodeId>,
    store: Vec<Option<NodeData>>,
}

impl DefaultNodeStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
            store: Vec::new(),
        }
    }
}

impl NodeStore for DefaultNodeStore {
    fn id_by_name<T: AsRef<str>>(&mut self, s: T) -> NodeId {
        self.interner.get_or_intern(s)
    }

    fn node_opt(&self, id: NodeId) -> Option<&NodeData> {
        self.store.get(id.to_usize())?.as_ref()
    }

    fn store_node(&mut self, id: NodeId, data: NodeData) {
        let id = id.to_usize();
        if self.store.len() <= id {
            self.store.resize(id + 1, None)
        }
        debug_assert!(self.store[id].is_none());
        self.store[id] = Some(data);
    }

    fn visit_nodes(&self, mut f: impl FnMut(&NodeData)) {
        for data in &self.store {
            if let Some(data) = data {
                f(data);
            }
        }
    }
}

impl Default for DefaultNodeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(u32);

impl ValueId {
    #[must_use]
    pub fn from_u32(i: u32) -> Self {
        Self(i)
    }
}

macro_rules! declare_value_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name(u32);

        impl From<$name> for ValueId {
            fn from(v: $name) -> ValueId {
                ValueId(v.0)
            }
        }

        impl From<ValueId> for $name {
            fn from(id: ValueId) -> Self {
                Self(id.0)
            }
        }
    };
}

declare_value_id!(IntegerId);
declare_value_id!(FloatId);
declare_value_id!(StringId);
declare_value_id!(BooleanId);

#[derive(Debug, Clone, PartialEq)]
pub enum ValueData {
    Integer(i64),
    Float(f64),
    Str(String),
    Boolean(bool),
}

pub trait ValueStore {
    fn store<T>(&mut self, data: impl Into<ValueData>) -> T
    where
        T: From<ValueId>;

    fn value_opt(&self, id: impl Into<ValueId>) -> Option<&ValueData>;

    fn value_mut_opt(&mut self, id: impl Into<ValueId>) -> Option<&mut ValueData>;

    fn value(&self, id: impl Into<ValueId>) -> &ValueData {
        self.value_opt(id).unwrap()
    }

    fn value_mut(&mut self, id: impl Into<ValueId>) -> &mut ValueData {
        self.value_mut_opt(id).unwrap()
    }

    fn update(&mut self, id: impl Into<ValueId>, value: impl Into<ValueData>) -> Option<ValueData> {
        let mut prev = self.value_mut_opt(id)?;
        Some(std::mem::replace(&mut prev, value.into()))
    }

    fn integer_value(&self, id: IntegerId) -> Option<i64> {
        if let ValueData::Integer(i) = self.value_opt(id)? {
            Some(*i)
        } else {
            None
        }
    }

    fn integer_value_mut(&mut self, id: IntegerId) -> Option<&mut i64> {
        if let ValueData::Integer(i) = self.value_mut_opt(id)? {
            Some(i)
        } else {
            None
        }
    }

    fn float_value(&self, id: FloatId) -> Option<f64> {
        if let ValueData::Float(f) = self.value_opt(id)? {
            Some(*f)
        } else {
            None
        }
    }

    fn float_value_mut(&mut self, id: FloatId) -> Option<&mut f64> {
        if let ValueData::Float(f) = self.value_mut_opt(id)? {
            Some(f)
        } else {
            None
        }
    }

    fn str_value(&self, id: StringId) -> Option<&str> {
        if let ValueData::Str(s) = self.value_opt(id)? {
            Some(s)
        } else {
            None
        }
    }

    fn boolean_value(&self, id: BooleanId) -> Option<bool> {
        if let ValueData::Boolean(b) = self.value_opt(id)? {
            Some(*b)
        } else {
            None
        }
    }

    fn boolean_value_mut(&mut self, id: BooleanId) -> Option<&mut bool> {
        if let ValueData::Boolean(b) = self.value_mut_opt(id)? {
            Some(b)
        } else {
            None
        }
    }

    fn str_value_mut(&mut self, id: StringId) -> Option<&mut str> {
        if let ValueData::Str(s) = self.value_mut_opt(id)? {
            Some(s)
        } else {
            None
        }
    }
}

impl<T> ValueStore for &mut T
where
    T: ValueStore,
{
    fn store<U>(&mut self, data: impl Into<ValueData>) -> U
    where
        U: From<ValueId>,
    {
        (*self).store(data)
    }

    fn value_opt(&self, id: impl Into<ValueId>) -> Option<&ValueData> {
        (**self).value_opt(id)
    }

    fn value_mut_opt(&mut self, id: impl Into<ValueId>) -> Option<&mut ValueData> {
        (*self).value_mut_opt(id)
    }
}

macro_rules! impl_value_data_conversion {
    ($ty:ty, $ctor:expr) => {
        impl From<$ty> for ValueData {
            fn from(v: $ty) -> Self {
                $ctor(v)
            }
        }
    };
}

impl_value_data_conversion!(i64, Self::Integer);
impl_value_data_conversion!(f64, Self::Float);
impl_value_data_conversion!(String, Self::Str);
impl_value_data_conversion!(bool, Self::Boolean);

#[derive(Debug, Default)]
pub struct DefaultValueStore(Vec<ValueData>);

impl DefaultValueStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ValueStore for DefaultValueStore {
    fn store<T>(&mut self, data: impl Into<ValueData>) -> T
    where
        T: From<ValueId>,
    {
        let id = u32::try_from(self.0.len())
            .expect("the number of value stored in `ValueStore` must not exceed u32::MAX");
        let id = ValueId(id);
        self.0.push(data.into());
        id.into()
    }

    fn value_opt(&self, id: impl Into<ValueId>) -> Option<&ValueData> {
        self.0.get(id.into().0 as usize)
    }

    fn value_mut_opt(&mut self, id: impl Into<ValueId>) -> Option<&mut ValueData> {
        self.0.get_mut(id.into().0 as usize)
    }
}

pub trait CacheStore {
    fn store(&mut self, node_id: NodeId, value_id: impl Into<ValueId>);

    fn value(&self, node_id: NodeId) -> Option<CacheData>;

    fn invalidate_by(&mut self, id: NodeId);
}

#[derive(Debug, Clone, Copy)]
pub struct CacheData {
    pub value_id: ValueId,
    pub is_valid: bool,
}

impl CacheData {
    fn new(value_id: ValueId, is_valid: bool) -> Self {
        Self { value_id, is_valid }
    }
}

#[derive(Debug)]
pub struct DefaultCacheStore {
    store: HashMap<NodeId, CacheData>,
    invalidators: HashMap<NodeId, Vec<NodeId>>,
}

impl<T> CacheStore for &mut T
where
    T: CacheStore,
{
    fn store(&mut self, node_id: NodeId, value_id: impl Into<ValueId>) {
        (**self).store(node_id, value_id);
    }

    fn value(&self, node_id: NodeId) -> Option<CacheData> {
        (**self).value(node_id)
    }

    fn invalidate_by(&mut self, id: NodeId) {
        (**self).invalidate_by(id)
    }
}

impl DefaultCacheStore {
    #[must_use]
    pub fn new(node_store: &impl NodeStore) -> Self {
        let mut invalidators: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        macro_rules! push_invalidators {
            ($node:ident) => {{
                let id = $node.node_base().id();
                for invalidator in $node.register_base().p_invalidators() {
                    let entry = invalidators.entry(*invalidator).or_default();
                    entry.push(id);
                }
            }};
        }
        node_store.visit_nodes(|node| match node {
            NodeData::IntReg(n) => push_invalidators!(n),
            NodeData::MaskedIntReg(n) => push_invalidators!(n),
            NodeData::FloatReg(n) => push_invalidators!(n),
            NodeData::StringReg(n) => push_invalidators!(n),
            NodeData::Register(n) => push_invalidators!(n),
            _ => {}
        });

        Self {
            store: HashMap::new(),
            invalidators,
        }
    }
}

impl CacheStore for DefaultCacheStore {
    fn store(&mut self, node_id: NodeId, value_id: impl Into<ValueId>) {
        self.store
            .entry(node_id)
            .and_modify(|e| e.is_valid = true)
            .or_insert(CacheData::new(value_id.into(), true));
    }

    fn value(&self, node_id: NodeId) -> Option<CacheData> {
        self.store.get(&node_id).copied()
    }

    fn invalidate_by(&mut self, id: NodeId) {
        if let Some(target_nodes) = self.invalidators.get(&id) {
            for n in target_nodes {
                if let Some(CacheData {
                    value_id: _,
                    is_valid,
                }) = self.store.get_mut(&n)
                {
                    *is_valid = false;
                }
            }
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct CacheSink {
    _priv: (),
}

impl CacheSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CacheStore for CacheSink {
    fn store(&mut self, _: NodeId, _: impl Into<ValueId>) {}

    fn value(&self, _: NodeId) -> Option<CacheData> {
        None
    }

    fn invalidate_by(&mut self, _: NodeId) {}
}
