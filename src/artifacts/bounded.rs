use std::{cell::Cell, fmt, mem::size_of};

use serde::{
    Deserializer,
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
};

use super::codec::{DecodeLimits, ResourceLimit};

pub(super) struct Budget {
    limits: DecodeLimits,
    entries: Cell<usize>,
    allocation: Cell<usize>,
    pub failure: Cell<Option<ResourceLimit>>,
}

impl Budget {
    pub fn new(limits: DecodeLimits) -> Self {
        Self {
            limits,
            entries: Cell::new(0),
            allocation: Cell::new(0),
            failure: Cell::new(None),
        }
    }

    pub fn reserve_workspace(&self, bytes: usize) -> Result<(), ResourceLimit> {
        let next = self
            .allocation
            .get()
            .checked_add(bytes)
            .filter(|next| *next <= self.limits.max_allocation_bytes)
            .ok_or(ResourceLimit::AllocationBytes)?;
        self.allocation.set(next);
        Ok(())
    }

    fn fail<Error: de::Error>(&self, limit: ResourceLimit) -> Error {
        if self.failure.get().is_none() {
            self.failure.set(Some(limit));
        }
        Error::custom(format!("artifact limit exceeded: {limit:?}"))
    }

    fn add<Error: de::Error>(
        &self,
        counter: &Cell<usize>,
        amount: usize,
        maximum: usize,
        limit: ResourceLimit,
    ) -> Result<(), Error> {
        let next = counter
            .get()
            .checked_add(amount)
            .filter(|next| *next <= maximum)
            .ok_or_else(|| self.fail(limit))?;
        counter.set(next);
        Ok(())
    }

    fn allocate<Error: de::Error>(&self, bytes: usize) -> Result<(), Error> {
        self.add(
            &self.allocation,
            bytes,
            self.limits.max_allocation_bytes,
            ResourceLimit::AllocationBytes,
        )
    }

    fn entry<Value, Error: de::Error>(&self, index: Option<usize>) -> Result<(), Error> {
        self.add(
            &self.entries,
            1,
            self.limits.max_total_entries,
            ResourceLimit::TotalEntries,
        )?;
        if let Some(index) = index {
            if index >= self.limits.max_collection_entries {
                return Err(self.fail(ResourceLimit::CollectionEntries));
            }
            let slots = if index == 0 { 8 } else { 2 };
            let charge = size_of::<Value>()
                .max(1)
                .checked_mul(slots)
                .ok_or_else(|| self.fail(ResourceLimit::AllocationBytes))?;
            self.allocate(charge)?;
        }
        Ok(())
    }

    fn string<Error: de::Error>(&self, length: usize) -> Result<(), Error> {
        if length > self.limits.max_string_bytes {
            return Err(self.fail(ResourceLimit::StringBytes));
        }
        self.allocate(
            length
                .checked_mul(2)
                .ok_or_else(|| self.fail(ResourceLimit::AllocationBytes))?,
        )
    }

    fn bytes<Error: de::Error>(&self, length: usize) -> Result<(), Error> {
        if length > self.limits.max_collection_entries {
            return Err(self.fail(ResourceLimit::CollectionEntries));
        }
        self.add(
            &self.entries,
            length,
            self.limits.max_total_entries,
            ResourceLimit::TotalEntries,
        )?;
        self.allocate(
            length
                .checked_mul(2)
                .ok_or_else(|| self.fail(ResourceLimit::AllocationBytes))?,
        )
    }
}

pub(super) struct Bounded<'budget, Inner> {
    pub inner: Inner,
    pub budget: &'budget Budget,
    pub depth: usize,
}

struct BoundedVisitor<'budget, Inner> {
    inner: Inner,
    budget: &'budget Budget,
    depth: usize,
    collection: bool,
}

impl<'budget, Inner> Bounded<'budget, Inner> {
    fn visitor<Visit, Error: de::Error>(
        &self,
        inner: Visit,
        collection: bool,
    ) -> Result<BoundedVisitor<'budget, Visit>, Error> {
        if self.depth >= self.budget.limits.max_depth {
            return Err(self.budget.fail(ResourceLimit::Depth));
        }
        Ok(BoundedVisitor {
            inner,
            budget: self.budget,
            depth: self.depth,
            collection,
        })
    }
}

macro_rules! deserialize_methods {
    ($($method:ident),* $(,)?) => {$(
        fn $method<Visit: Visitor<'de>>(self, visitor: Visit) -> Result<Visit::Value, Self::Error> {
            let visitor = self.visitor(visitor, false)?;
            self.inner.$method(visitor)
        }
    )*};
}

impl<'de, Inner: Deserializer<'de>> Deserializer<'de> for Bounded<'_, Inner> {
    type Error = Inner::Error;

    deserialize_methods!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit
    );

    fn deserialize_ignored_any<Visit: Visitor<'de>>(
        self,
        _: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        Err(de::Error::custom("unknown field in artifact record"))
    }

    fn deserialize_identifier<Visit: Visitor<'de>>(
        self,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        self.inner.deserialize_identifier(visitor)
    }

    fn deserialize_string<Visit: Visitor<'de>>(
        self,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_seq<Visit: Visitor<'de>>(
        self,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, true)?;
        self.inner.deserialize_seq(visitor)
    }

    fn deserialize_map<Visit: Visitor<'de>>(
        self,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, true)?;
        self.inner.deserialize_map(visitor)
    }

    fn deserialize_unit_struct<Visit: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<Visit: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_tuple<Visit: Visitor<'de>>(
        self,
        length: usize,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_tuple(length, visitor)
    }

    fn deserialize_tuple_struct<Visit: Visitor<'de>>(
        self,
        name: &'static str,
        length: usize,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_tuple_struct(name, length, visitor)
    }

    fn deserialize_struct<Visit: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<Visit: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.deserialize_enum(name, variants, visitor)
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

macro_rules! visit_values {
    ($($method:ident: $value:ty),* $(,)?) => {$(
        fn $method<Error: de::Error>(self, value: $value) -> Result<Self::Value, Error> {
            self.inner.$method(value)
        }
    )*};
}

impl<'de, Inner: Visitor<'de>> Visitor<'de> for BoundedVisitor<'_, Inner> {
    type Value = Inner::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    visit_values!(visit_bool: bool, visit_i8: i8, visit_i16: i16, visit_i32: i32, visit_i64: i64,
        visit_i128: i128, visit_u8: u8, visit_u16: u16, visit_u32: u32, visit_u64: u64,
        visit_u128: u128, visit_f32: f32, visit_f64: f64, visit_char: char);

    fn visit_str<Error: de::Error>(self, value: &str) -> Result<Self::Value, Error> {
        self.budget.string(value.len())?;
        self.inner.visit_str(value)
    }

    fn visit_borrowed_str<Error: de::Error>(self, value: &'de str) -> Result<Self::Value, Error> {
        self.budget.string(value.len())?;
        self.inner.visit_borrowed_str(value)
    }

    fn visit_string<Error: de::Error>(self, value: String) -> Result<Self::Value, Error> {
        self.budget.string(value.len())?;
        self.inner.visit_string(value)
    }

    fn visit_bytes<Error: de::Error>(self, value: &[u8]) -> Result<Self::Value, Error> {
        self.budget.bytes(value.len())?;
        self.inner.visit_bytes(value)
    }

    fn visit_borrowed_bytes<Error: de::Error>(
        self,
        value: &'de [u8],
    ) -> Result<Self::Value, Error> {
        self.budget.bytes(value.len())?;
        self.inner.visit_borrowed_bytes(value)
    }

    fn visit_byte_buf<Error: de::Error>(self, value: Vec<u8>) -> Result<Self::Value, Error> {
        self.budget.bytes(value.len())?;
        self.inner.visit_byte_buf(value)
    }

    fn visit_unit<Error: de::Error>(self) -> Result<Self::Value, Error> {
        self.inner.visit_unit()
    }
    fn visit_none<Error: de::Error>(self) -> Result<Self::Value, Error> {
        self.inner.visit_none()
    }

    fn visit_some<Deser: Deserializer<'de>>(
        self,
        inner: Deser,
    ) -> Result<Self::Value, Deser::Error> {
        self.inner.visit_some(Bounded {
            inner,
            budget: self.budget,
            depth: self.depth + 1,
        })
    }

    fn visit_newtype_struct<Deser: Deserializer<'de>>(
        self,
        inner: Deser,
    ) -> Result<Self::Value, Deser::Error> {
        self.inner.visit_newtype_struct(Bounded {
            inner,
            budget: self.budget,
            depth: self.depth + 1,
        })
    }

    fn visit_seq<Access: SeqAccess<'de>>(
        self,
        inner: Access,
    ) -> Result<Self::Value, Access::Error> {
        if self.collection
            && inner
                .size_hint()
                .is_some_and(|size| size > self.budget.limits.max_collection_entries)
        {
            return Err(self.budget.fail(ResourceLimit::CollectionEntries));
        }
        self.inner.visit_seq(BoundedAccess {
            inner,
            budget: self.budget,
            depth: self.depth + 1,
            count: 0,
            collection: self.collection,
        })
    }

    fn visit_map<Access: MapAccess<'de>>(
        self,
        inner: Access,
    ) -> Result<Self::Value, Access::Error> {
        if self.collection
            && inner
                .size_hint()
                .is_some_and(|size| size > self.budget.limits.max_collection_entries)
        {
            return Err(self.budget.fail(ResourceLimit::CollectionEntries));
        }
        self.inner.visit_map(BoundedAccess {
            inner,
            budget: self.budget,
            depth: self.depth + 1,
            count: 0,
            collection: self.collection,
        })
    }

    fn visit_enum<Access: EnumAccess<'de>>(
        self,
        inner: Access,
    ) -> Result<Self::Value, Access::Error> {
        self.inner.visit_enum(Bounded {
            inner,
            budget: self.budget,
            depth: self.depth + 1,
        })
    }
}

struct BoundedSeed<'budget, Inner> {
    inner: Inner,
    budget: &'budget Budget,
    depth: usize,
    index: Option<usize>,
}

impl<'de, Inner: DeserializeSeed<'de>> DeserializeSeed<'de> for BoundedSeed<'_, Inner> {
    type Value = Inner::Value;

    fn deserialize<Deser: Deserializer<'de>>(
        self,
        inner: Deser,
    ) -> Result<Self::Value, Deser::Error> {
        self.budget
            .entry::<Inner::Value, Deser::Error>(self.index)?;
        self.inner.deserialize(Bounded {
            inner,
            budget: self.budget,
            depth: self.depth,
        })
    }
}

struct BoundedAccess<'budget, Inner> {
    inner: Inner,
    budget: &'budget Budget,
    depth: usize,
    count: usize,
    collection: bool,
}

impl<'de, Inner: SeqAccess<'de>> SeqAccess<'de> for BoundedAccess<'_, Inner> {
    type Error = Inner::Error;

    fn next_element_seed<Seed: DeserializeSeed<'de>>(
        &mut self,
        seed: Seed,
    ) -> Result<Option<Seed::Value>, Self::Error> {
        let value = self.inner.next_element_seed(BoundedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            index: self.collection.then_some(self.count),
        })?;
        if value.is_some() {
            self.count += 1;
        }
        Ok(value)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

impl<'de, Inner: MapAccess<'de>> MapAccess<'de> for BoundedAccess<'_, Inner> {
    type Error = Inner::Error;

    fn next_key_seed<Seed: DeserializeSeed<'de>>(
        &mut self,
        seed: Seed,
    ) -> Result<Option<Seed::Value>, Self::Error> {
        if !self.collection {
            return self.inner.next_key_seed(seed);
        }
        self.inner.next_key_seed(BoundedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            index: self.collection.then_some(self.count),
        })
    }

    fn next_value_seed<Seed: DeserializeSeed<'de>>(
        &mut self,
        seed: Seed,
    ) -> Result<Seed::Value, Self::Error> {
        let value = self.inner.next_value_seed(BoundedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            index: self.collection.then_some(self.count),
        })?;
        self.count += 1;
        Ok(value)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

impl<'de, 'budget, Inner: EnumAccess<'de>> EnumAccess<'de> for Bounded<'budget, Inner> {
    type Error = Inner::Error;
    type Variant = Bounded<'budget, Inner::Variant>;

    fn variant_seed<Seed: DeserializeSeed<'de>>(
        self,
        seed: Seed,
    ) -> Result<(Seed::Value, Self::Variant), Self::Error> {
        let (value, inner) = self.inner.variant_seed(BoundedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            index: None,
        })?;
        Ok((
            value,
            Bounded {
                inner,
                budget: self.budget,
                depth: self.depth,
            },
        ))
    }
}

impl<'de, Inner: VariantAccess<'de>> VariantAccess<'de> for Bounded<'_, Inner> {
    type Error = Inner::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.inner.unit_variant()
    }

    fn newtype_variant_seed<Seed: DeserializeSeed<'de>>(
        self,
        seed: Seed,
    ) -> Result<Seed::Value, Self::Error> {
        self.inner.newtype_variant_seed(BoundedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth + 1,
            index: None,
        })
    }

    fn tuple_variant<Visit: Visitor<'de>>(
        self,
        length: usize,
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.tuple_variant(length, visitor)
    }

    fn struct_variant<Visit: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: Visit,
    ) -> Result<Visit::Value, Self::Error> {
        let visitor = self.visitor(visitor, false)?;
        self.inner.struct_variant(fields, visitor)
    }
}
