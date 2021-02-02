use super::super::{Consume, PartitionWriter, Writer};
use crate::data_order::DataOrder;
use crate::errors::{ConnectorAgentError, Result};
use crate::types::DataType;
use crate::typesystem::{TypeAssoc, TypeSystem};
use anyhow::anyhow;
use fehler::{throw, throws};
use ndarray::{Array2, ArrayView2, ArrayViewMut2, Axis};
use std::mem::transmute;

/// This `Writer` can only write bool into it.
#[derive(Clone)]
pub struct BoolWriter {
    nrows: usize,
    schema: Vec<DataType>,
    buffer: Array2<bool>,
}

impl BoolWriter {
    pub fn buffer(&self) -> ArrayView2<bool> {
        self.buffer.view()
    }
}

impl<'a> Writer<'a> for BoolWriter {
    const DATA_ORDERS: &'static [DataOrder] = &[DataOrder::RowMajor];
    type TypeSystem = DataType;
    type PartitionWriter = BoolPartitionWriter<'a>;

    #[throws(ConnectorAgentError)]
    fn allocate(nrows: usize, schema: Vec<DataType>, data_order: DataOrder) -> Self {
        let ncols = schema.len();
        for field in &schema {
            if !matches!(field, DataType::Bool) {
                throw!(anyhow!("BoolWriter only accepts Bool only schema"));
            }
        }
        if !matches!(data_order, DataOrder::RowMajor) {
            throw!(ConnectorAgentError::UnsupportedDataOrder(data_order))
        }

        BoolWriter {
            nrows,
            schema,
            buffer: Array2::from_elem((nrows, ncols), false),
        }
    }

    fn partition_writers(&'a mut self, counts: &[usize]) -> Vec<Self::PartitionWriter> {
        assert_eq!(counts.iter().sum::<usize>(), self.nrows);
        let schema = self.schema().to_vec();

        let mut mut_view = self.buffer.view_mut();
        let mut ret = vec![];
        for &c in counts {
            let (splitted, rest) = mut_view.split_at(Axis(0), c);
            mut_view = rest;
            ret.push(BoolPartitionWriter::new(splitted, schema.clone()));
        }
        ret
    }

    fn schema(&self) -> &[DataType] {
        self.schema.as_slice()
    }
}

/// The `PartitionedWriter` of `BoolWriter`.
pub struct BoolPartitionWriter<'a> {
    buffer: ArrayViewMut2<'a, bool>,
    schema: Vec<DataType>,
}

impl<'a> BoolPartitionWriter<'a> {
    fn new(buffer: ArrayViewMut2<'a, bool>, schema: Vec<DataType>) -> Self {
        Self { buffer, schema }
    }
}

impl<'a> PartitionWriter<'a> for BoolPartitionWriter<'a> {
    type TypeSystem = DataType;

    fn nrows(&self) -> usize {
        self.buffer.nrows()
    }

    fn ncols(&self) -> usize {
        self.buffer.ncols()
    }
}

impl<'a, T> Consume<T> for BoolPartitionWriter<'a>
where
    T: TypeAssoc<<Self as PartitionWriter<'a>>::TypeSystem> + 'static,
{
    unsafe fn consume(&mut self, row: usize, col: usize, value: T) {
        let target: *mut T = transmute(self.buffer.uget_mut((row, col)));
        *target = value;
    }

    fn consume_checked(&mut self, row: usize, col: usize, value: T) -> Result<()> {
        self.schema[col].check::<T>()?;
        unsafe { self.write(row, col, value) };
        Ok(())
    }
}
