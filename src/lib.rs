#![feature(specialization)]

extern crate densearray;
#[cfg(cuda)] extern crate devicemem_cuda;

pub use DataKind::{Val, Grad, RVal, RGrad};

//use arithmetic::*;
//use densearray::prelude::*;

use std::any::{Any};
use std::cell::{Cell, RefCell, Ref, RefMut};
use std::collections::{HashSet};
use std::rc::{Rc};

pub mod ops;
#[cfg(cuda)] pub mod ops_cuda;
pub mod prelude;

thread_local!(static NODE_ID_COUNTER: Cell<u64> = Cell::new(0));
thread_local!(static TXN_ID_COUNTER:  Cell<u64> = Cell::new(0));
thread_local!(static EPOCH_COUNTER:   Cell<u64> = Cell::new(0));

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(u64);

impl NodeId {
  pub fn new() -> NodeId {
    NODE_ID_COUNTER.with(|counter| {
      let prev_count = counter.get();
      counter.set(prev_count + 1);
      let next_count = counter.get();
      assert_eq!(next_count, prev_count + 1);
      assert!(next_count != 0);
      NodeId(next_count)
    })
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TxnId(u64);

impl TxnId {
  pub fn new() -> TxnId {
    TXN_ID_COUNTER.with(|counter| {
      let prev_count = counter.get();
      counter.set(prev_count + 1);
      let next_count = counter.get();
      assert_eq!(next_count, prev_count + 1);
      assert!(next_count != 0);
      TxnId(next_count)
    })
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EpochNr(u64);

impl EpochNr {
  pub fn new() -> EpochNr {
    EPOCH_COUNTER.with(|counter| {
      let prev_count = counter.get();
      counter.set(prev_count + 1);
      let next_count = counter.get();
      assert_eq!(next_count, prev_count + 1);
      assert!(next_count != 0);
      EpochNr(next_count)
    })
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Epoch {
  pub root:     NodeId,
  pub epoch_nr: EpochNr,
}

impl Epoch {
  pub fn new(root: NodeId) -> Epoch {
    Epoch{
      root:     root,
      epoch_nr: EpochNr::new(),
    }
  }
}

pub struct OperatorStackEntry {
  epoch:        Epoch,
  push_count:   usize,
  pop_count:    usize,
}

pub struct OperatorStack {
  node_id:      NodeId,
  in_degree:    usize,
  entries:      RefCell<Vec<OperatorStackEntry>>,
}

impl OperatorStack {
  pub fn new(node_id: NodeId, in_degree: usize) -> OperatorStack {
    OperatorStack{
      node_id:      node_id,
      in_degree:    in_degree,
      entries:      RefCell::new(vec![]),
    }
  }

  pub fn push(&self, epoch: Epoch) -> usize {
    let mut entries = self.entries.borrow_mut();
    if !entries.is_empty() && epoch == entries.last().unwrap().epoch {
      entries.last_mut().unwrap().push_count += 1;
    } else {
      entries.push(OperatorStackEntry{
        epoch:      epoch,
        push_count: 1,
        pop_count:  0,
      });
    }
    entries.last().unwrap().push_count
  }

  pub fn degree(&self, epoch: Epoch) -> usize {
    let mut entries = self.entries.borrow();
    assert!(!entries.is_empty());
    let level = entries.len() - 1;
    assert_eq!(epoch, entries[level].epoch);
    entries[level].push_count
  }

  pub fn pop(&self, epoch: Epoch) -> usize {
    let mut entries = self.entries.borrow_mut();
    assert!(!entries.is_empty());
    let level = entries.len() - 1;
    assert_eq!(epoch, entries[level].epoch);
    entries[level].pop_count += 1;
    let saved_pop_count = entries[level].pop_count;
    if entries[level].push_count == entries[level].pop_count {
      entries.pop();
    }
    saved_pop_count
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataKind {
  Val,
  Grad,
  RVal,
  RGrad,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DataRef {
  pub node_id:  NodeId,
  pub kind:     DataKind,
}

impl DataRef {
  /*pub fn new() -> DataRef {
    DataRef{node_id: NodeId::new()}
  }*/

  pub fn new(kind: DataKind) -> DataRef {
    DataRef{
      node_id:  NodeId::new(),
      kind:     kind,
    }
  }
}

#[derive(Clone)]
pub struct DataRefSet {
  inner:    HashSet<DataRef>,
  mask:     HashSet<DataRef>,
}

impl DataRefSet {
  pub fn empty() -> DataRefSet {
    DataRefSet{
      inner:    HashSet::new(),
      mask:     HashSet::new(),
    }
  }

  pub fn add(mut self, data_ref: DataRef) -> Self {
    self.inner.insert(data_ref);
    self
  }

  pub fn clear(&mut self) {
    self.inner.clear();
    self.mask.clear();
  }

  pub fn contains(&mut self, data_ref: &DataRef) -> bool {
    self.inner.contains(data_ref)
  }

  pub fn unmask_all(&mut self) {
    self.mask.clear();
  }

  pub fn mask(&mut self, data_ref: DataRef) {
    self.mask.insert(data_ref);
  }

  pub fn is_masked(&mut self, data_ref: &DataRef) -> bool {
    self.mask.contains(data_ref)
  }
}

pub trait DiffOperator {
  fn _id(&self) -> NodeId;
  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&DiffOperator));
  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&DiffOperator));

  fn _data_sz(&self, ref_set: &mut DataRefSet) -> usize { 0 }
  fn _load(&self, txn: TxnId, ref_set: &mut DataRefSet, reader: &mut Any) {}
  fn _load_r_val(&self, txn: TxnId, ref_set: &mut DataRefSet, reader: &mut Any) {}
  fn _store(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {}
  fn _store_grad(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {}
  fn _store_r_grad(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {}
  fn _rollover(&self, txn: TxnId, ref_set: &mut DataRefSet);

  //fn _clear(&self, txn: TxnId) {}
  fn _init(&self, txn: TxnId) {}
  fn _forward(&self, txn: TxnId);
  fn _backward(&self, txn: TxnId, gauss_newton: bool) { unimplemented!(); }
  fn _r_forward(&self, txn: TxnId, gauss_newton: bool) { unimplemented!(); }
  fn _r_backward(&self, txn: TxnId) { unimplemented!(); }
  fn _clock(&self, txn: TxnId) { unimplemented!(); }

  fn data_sz(&self, ref_set: &mut DataRefSet) -> usize {
    let epoch = Epoch::new(self._id());
    let mut size = 0;
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { size += op._data_sz(ref_set); });
    size
  }

  fn load(&self, txn: TxnId, ref_set: &mut DataRefSet, reader: &mut Any) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._load(txn, ref_set, reader); });
    ref_set.unmask_all();
  }

  fn load_r_val(&self, txn: TxnId, ref_set: &mut DataRefSet, reader: &mut Any) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._load_r_val(txn, ref_set, reader); });
    ref_set.unmask_all();
  }

  fn store(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._store(txn, ref_set, writer); });
    ref_set.unmask_all();
  }

  fn store_grad(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._store_grad(txn, ref_set, writer); });
    ref_set.unmask_all();
  }

  fn store_r_grad(&self, txn: TxnId, ref_set: &mut DataRefSet, writer: &mut Any) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._store_r_grad(txn, ref_set, writer); });
    ref_set.unmask_all();
  }

  fn init(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._init(txn); });
  }

  /*fn clear(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._push(epoch, &mut |op| { op._clear(txn); });
    self._pop(epoch, &mut |_op| {});
  }*/

  fn rollover(&self, txn: TxnId, ref_set: &mut DataRefSet) {
    let epoch = Epoch::new(self._id());
    ref_set.unmask_all();
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._rollover(txn, ref_set); });
    ref_set.unmask_all();
  }

  fn eval(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._push(epoch, &mut |op| { op._forward(txn); });
    self._pop(epoch, &mut |_op| {});
  }

  fn clock(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._push(epoch, &mut |op| { op._clock(txn); });
    self._pop(epoch, &mut |_op| {});
  }
}

pub trait DiffScalarLoss: DiffOperator {
  fn _set_source(&self, txn: TxnId);

  fn gradient(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._set_source(txn);
    self._push(epoch, &mut |op| { op._forward(txn); });
    self._pop(epoch, &mut |_op| {});
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._backward(txn, /*ref_set,*/ false); });
  }

  fn gauss_newton_vector_product(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._set_source(txn);
    self._push(epoch, &mut |op| { op._forward(txn); });
    self._pop(epoch, &mut |_op| {});
    self._push(epoch, &mut |op| { op._r_forward(txn, /*ref_set,*/ true); });
    self._pop(epoch, &mut |_op| {});
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._backward(txn, /*ref_set,*/ true); });
  }

  fn hessian_vector_product(&self, txn: TxnId) {
    let epoch = Epoch::new(self._id());
    self._set_source(txn);
    self._push(epoch, &mut |op| { op._forward(txn); });
    self._pop(epoch, &mut |_op| {});
    self._push(epoch, &mut |op| { op._r_forward(txn, /*ref_set,*/ false); });
    self._pop(epoch, &mut |_op| {});
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._backward(txn, /*ref_set,*/ false); });
    self._push(epoch, &mut |_op| {});
    self._pop(epoch, &mut |op| { op._r_backward(txn, /*ref_set*/); });
  }
}

pub struct FlatReader<A> {
  buffer:   A,
  offset:   usize,
}

pub struct FlatWriter<A> {
  buffer:   A,
  offset:   usize,
}

pub trait ArrayOp<A>: DiffOperator {
  fn data(&self) -> Rc<ArrayData<A>>;
}

pub trait ArrayStorage<Idx> {
  fn alloc(dim: Idx) -> Self where Self: Sized;
}

impl ArrayStorage<usize> for Vec<f32> {
  fn alloc(dim: usize) -> Self {
    let mut buf = Vec::with_capacity(dim);
    buf.resize(dim, 0.0);
    buf
  }
}

pub struct TxnDataBuf<A> {
  clk:      usize,
  curr_txn: Cell<Option<TxnId>>,
  readers:  RefCell<HashSet<NodeId>>,
  writers:  RefCell<HashSet<NodeId>>,
  buffer:   RefCell<Option<A>>,
}

impl<A> TxnDataBuf<A> {
  pub fn new(clk: usize) -> Self {
    TxnDataBuf{
      clk:      clk,
      curr_txn: Cell::new(None),
      readers:  RefCell::new(HashSet::new()),
      writers:  RefCell::new(HashSet::new()),
      buffer:   RefCell::new(None),
    }
  }
}

pub struct TxnData<A> {
  data_ref: DataRef,
  kind:     DataKind,
  alloc:    Rc<Fn(TxnId, NodeId) -> A>,
  curr_clk: Cell<usize>,
  clk_bufs: Vec<TxnDataBuf<A>>,
}

impl<A> TxnData<A> {
  pub fn new(kind: DataKind, clk_horizon: usize, alloc: Rc<Fn(TxnId, NodeId) -> A>) -> Self {
    let mut clk_bufs = Vec::with_capacity(clk_horizon);
    for clk in 0 .. clk_horizon {
      clk_bufs.push(TxnDataBuf::new(clk));
    }
    TxnData{
      data_ref: DataRef::new(kind),
      kind:     kind,
      alloc:    alloc,
      curr_clk: Cell::new(0),
      clk_bufs: clk_bufs,
    }
  }

  pub fn _ref(&self) -> DataRef {
    self.data_ref.clone()
  }

  pub fn hard_reset(&mut self) {
    self.curr_clk.set(0);
    self.clk_bufs.clear();
  }

  pub fn reset_clock(&self) {
    self.curr_clk.set(0);
  }

  pub fn step_clock(&self) {
    self.curr_clk.set(self.curr_clk.get() + 1);
  }

  pub fn rollover(&self, txn: TxnId, ref_set: &mut DataRefSet) {
    let clk = self.curr_clk.get();
    let buf = &self.clk_bufs[clk];
    if ref_set.contains(&self.data_ref) {
      buf.curr_txn.set(Some(txn));
    } else {
      buf.curr_txn.set(None);
    }
    buf.readers.borrow_mut().clear();
    buf.writers.borrow_mut().clear();
  }

  pub fn write<F>(&self, txn: TxnId, node: NodeId, init: F) -> bool where F: Fn(&mut A) {
    let clk = self.curr_clk.get();
    let buf = &self.clk_bufs[clk];
    let mut incomplete_write = false;
    let mut new_txn = false;
    if buf.curr_txn.get().is_none() {
      incomplete_write = true;
      new_txn = true;
    } else {
      let curr_txn = buf.curr_txn.get().unwrap();
      if curr_txn != txn {
        incomplete_write = true;
        new_txn = true;
      } else {
        incomplete_write = !buf.writers.borrow().contains(&node);
      }
    }
    if new_txn {
      buf.curr_txn.set(Some(txn));
      buf.readers.borrow_mut().clear();
      buf.writers.borrow_mut().clear();
      let mut buffer = buf.buffer.borrow_mut();
      if buffer.is_none() {
        *buffer = Some((self.alloc)(txn, node));
      }
      init(&mut *buffer.as_mut().unwrap());
    }
    incomplete_write
  }

  pub fn get(&self, txn: TxnId, node: NodeId) -> Ref<A> {
    let clk = self.curr_clk.get();
    let buf = &self.clk_bufs[clk];
    let mut new_txn = false;
    if buf.curr_txn.get().is_none() {
      new_txn = true;
    } else {
      let curr_txn = buf.curr_txn.get().unwrap();
      new_txn = curr_txn != txn;
    }
    assert!(!new_txn);
    assert!(!buf.writers.borrow().contains(&node));
    buf.readers.borrow_mut().insert(node);
    Ref::map(buf.buffer.borrow(), |buffer| {
      if let Some(buffer) = buffer.as_ref() {
        buffer
      } else {
        panic!("trying to read from unallocated buffer");
      }
    })
  }

  pub fn get_prev(&self, txn: TxnId, node: NodeId) -> Option<Ref<A>> {
    let clk = self.curr_clk.get();
    if clk == 0 {
      return None;
    }
    let buf = &self.clk_bufs[clk-1];
    let mut new_txn = false;
    if buf.curr_txn.get().is_none() {
      new_txn = true;
    } else {
      let curr_txn = buf.curr_txn.get().unwrap();
      new_txn = curr_txn != txn;
    }
    assert!(!new_txn);
    assert!(!buf.writers.borrow().contains(&node));
    buf.readers.borrow_mut().insert(node);
    Some(Ref::map(buf.buffer.borrow(), |buffer| {
      if let Some(buffer) = buffer.as_ref() {
        buffer
      } else {
        panic!("trying to read from unallocated buffer");
      }
    }))
  }

  pub fn get_mut(&self, txn: TxnId, node: NodeId) -> RefMut<A> {
    let clk = self.curr_clk.get();
    let buf = &self.clk_bufs[clk];
    let mut new_txn = false;
    if buf.curr_txn.get().is_none() {
      new_txn = true;
    } else {
      let curr_txn = buf.curr_txn.get().unwrap();
      new_txn = curr_txn != txn;
    }
    if new_txn {
      buf.curr_txn.set(Some(txn));
      buf.readers.borrow_mut().clear();
      buf.writers.borrow_mut().clear();
    }
    assert!(!buf.readers.borrow().contains(&node));
    buf.writers.borrow_mut().insert(node);
    RefMut::map(buf.buffer.borrow_mut(), |buf| {
      if let Some(buf) = buf.as_mut() {
        buf
      } else {
        panic!("trying to write to unallocated buffer");
      }
    })
  }
}

pub struct ArrayData</*Idx,*/ A> {
  //dim:          Idx,
  clk_horizon:  usize,
  alloc:        Rc<Fn(TxnId, NodeId) -> A>,
  pub val:      TxnData<A>,
  pub grad:     TxnData<A>,
  pub r_val:    TxnData<A>,
  pub r_grad:   TxnData<A>,
}

impl</*Idx,*/ A> ArrayData</*Idx,*/ A> {
  pub fn new(/*dim: Idx,*/ clk_horizon: usize, alloc: Rc<Fn(TxnId, NodeId) -> A>) -> Rc<Self> {
    Rc::new(ArrayData{
      //dim:      dim,
      clk_horizon:  clk_horizon,
      alloc:        alloc.clone(),
      val:      TxnData::new(Val, clk_horizon, alloc.clone()),
      grad:     TxnData::new(Grad, clk_horizon, alloc.clone()),
      r_val:    TxnData::new(RVal, clk_horizon, alloc.clone()),
      r_grad:   TxnData::new(RGrad, clk_horizon, alloc.clone()),
    })
  }

  pub fn refs(&self) -> (DataRef, DataRef, DataRef, DataRef) {
    unimplemented!();
  }

  pub fn horizon(&self) -> usize {
    self.clk_horizon
  }

  pub fn reset_all(&self) {
    self.val.reset_clock();
    self.grad.reset_clock();
    self.r_val.reset_clock();
    self.r_grad.reset_clock();
  }

  pub fn step_all(&self) {
    self.val.step_clock();
    self.grad.step_clock();
    self.r_val.step_clock();
    self.r_grad.step_clock();
  }

  pub fn rollover_all(&self, txn: TxnId, ref_set: &mut DataRefSet) {
    self.val.rollover(txn, ref_set);
    self.grad.rollover(txn, ref_set);
    self.r_val.rollover(txn, ref_set);
    self.r_grad.rollover(txn, ref_set);
  }
}
