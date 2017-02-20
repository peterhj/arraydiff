use prelude::*;
use ffi::*;
use ops::*;

use async_execution::*;
use cuda_dnn::v5::*;
use cuda_dnn::v5::ffi::*;
use densearray::prelude::*;
use devicemem_cuda::prelude::*;

use std::any::{Any};
use std::cell::{Cell, RefCell, Ref, RefMut};
use std::cmp::{max};
use std::collections::{HashMap};
use std::marker::{PhantomData};
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::sync::{Arc};

impl<'a, T> CursorBufExt<'a> for CursorBuf<DeviceMem<T>> where T: 'a + Copy {
  type Ref = DeviceMemRef<'a, T>;
  type Mut = DeviceMemRefMut<'a, T>;

  fn read_buf(&'a mut self, length: usize) -> DeviceMemRef<'a, T> {
    let start = self.offset;
    let end = self.offset + length;
    self.offset += length;
    self.buffer.as_ref().slice(start, end)
  }

  fn write_buf(&'a mut self, length: usize) -> DeviceMemRefMut<'a, T> {
    let start = self.offset;
    let end = self.offset + length;
    self.offset += length;
    self.buffer.as_mut().slice_mut(start, end)
  }
}

impl ArrayOp<DeviceBatchIoMem<u8>> for ArraySrc<DeviceBatchIoMem<u8>> {
  fn data(&self) -> ArrayData<DeviceBatchIoMem<u8>> {
    self.data.clone()
  }
}

impl AutodiffOp for ArraySrc<DeviceBatchIoMem<u8>> {
  fn _load_val(&self, txn: TxnId, vars: &mut VarSet, reader: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if self.data.val.overwrite(txn, node) {
        if reader.downcast_mut::<Vec<Arc<Deref<Target=[u8]>>>>().is_some() {
          let src_bufs = reader.downcast_mut::<Vec<Arc<Deref<Target=[u8]>>>>().unwrap();
          let mut val = self.data.val.get_excl(txn, node);
          let batch_sz = src_bufs.len();
          val.set_batch_size(batch_sz, &*DeviceStream::implicit());
          for idx in 0 .. batch_sz {
            val.load(idx, &**src_bufs[idx], DeviceStream::implicit().conn());
          }
        /*} else if reader.downcast_mut::<(usize, usize, Arc<Deref<Target=[u8]>>)>().is_some() {
          let &mut (ref batch_idx, ref batch_sz, ref src_mem) = reader.downcast_mut::<(usize, usize, Arc<Deref<Target=[u8]>>)>().unwrap();
          let mut val = self.data.val.get_mut(txn, node);
          let val_len = val.stride();
          val.set_batch_size(*batch_sz, &*DeviceStream::implicit());
          val.load(*batch_idx, &**src_mem, DeviceStream::implicit().conn());*/
        } else {
          unimplemented!();
        }
      }
    }
  }

  fn _store_val(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    //unimplemented!();
  }

  fn _store_grad(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    //unimplemented!();
  }

  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.data.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
  }

  fn _backward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.data.r_val.overwrite(txn, node) {
      // TODO: zero out the memory.
      //self.data.r_val.get_excl(txn, node)
      unimplemented!();
    }
  }

  fn _r_backward(&self, _txn: TxnId) {
  }

  fn _backward2(&self, _txn: TxnId) {
  }

  fn _reset_clock(&self) {
    if self.clock {
      self.data.reset_clock_all();
    }
  }

  fn _set_clock(&self, clk: usize) {
    if self.clock {
      self.data.set_clock_all(clk);
    }
  }
}

impl ArrayOp<DeviceArray1d<f32>> for ArraySrc<DeviceArray1d<f32>> {
  fn data(&self) -> ArrayData<DeviceArray1d<f32>> {
    self.data.clone()
  }
}

impl AutodiffOp for ArraySrc<DeviceArray1d<f32>> {
  fn _load_val(&self, txn: TxnId, vars: &mut VarSet, reader: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if self.data.val.overwrite(txn, node) {
        if reader.downcast_mut::<CursorBuf<Vec<f32>>>().is_some() {
          let mut val = self.data.val.get_mut(txn, node);
          let val_len = val.dim();
          let reader = reader.downcast_mut::<CursorBuf<Vec<f32>>>().unwrap();
          val.as_view_mut().load_sync(reader.read_buf(val_len).flatten(), DeviceStream::implicit().conn());
        } else if reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
          let mut val = self.data.val.get_mut(txn, node);
          let val_len = val.dim();
          let reader = reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
          val.as_view_mut().copy(reader.read_buf(val_len).flatten(), DeviceStream::implicit().conn());
        } else {
          unimplemented!();
        }
      }
    }
  }

  fn _store_val(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if writer.downcast_mut::<CursorBuf<Vec<f32>>>().is_some() {
        let mut val = self.data.val.get(txn, node);
        let val_len = val.dim();
        let writer = writer.downcast_mut::<CursorBuf<Vec<f32>>>().unwrap();
        val.as_view().store_sync(writer.write_buf(val_len).flatten_mut(), DeviceStream::implicit().conn());
      } else if writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
        let mut val = self.data.val.get(txn, node);
        let val_len = val.dim();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(val_len).flatten_mut().copy(val.as_view(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _store_grad(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.grad.var()) {
      if writer.downcast_mut::<CursorBuf<Vec<f32>>>().is_some() {
        let mut grad = self.data.grad.get(txn, node);
        let grad_len = grad.dim();
        let writer = writer.downcast_mut::<CursorBuf<Vec<f32>>>().unwrap();
        grad.as_view().store_sync(writer.write_buf(grad_len).flatten_mut(), DeviceStream::implicit().conn());
      } else if writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
        let mut grad = self.data.grad.get(txn, node);
        let grad_len = grad.dim();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(grad_len).flatten_mut().copy(grad.as_view(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.data.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
  }

  fn _backward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_forward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_backward(&self, _txn: TxnId) {
  }

  fn _reset_clock(&self) {
    if self.clock {
      self.data.reset_clock_all();
    }
  }

  fn _set_clock(&self, clk: usize) {
    if self.clock {
      self.data.set_clock_all(clk);
    }
  }
}

impl ArrayOp<DeviceArray2d<f32>> for ArraySrc<DeviceArray2d<f32>> {
  fn data(&self) -> ArrayData<DeviceArray2d<f32>> {
    self.data.clone()
  }
}

impl AutodiffOp for ArraySrc<DeviceArray2d<f32>> {
  fn _load_val(&self, txn: TxnId, vars: &mut VarSet, reader: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if self.data.val.overwrite(txn, node) {
        if reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
          let mut val = self.data.val.get_mut(txn, node);
          let val_len = val.dim().flat_len();
          let reader = reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
          val.as_view_mut().flatten_mut().copy(reader.read_buf(val_len).flatten(), DeviceStream::implicit().conn());
        } else {
          unimplemented!();
        }
      }
    }
  }

  fn _store_val(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
        let mut val = self.data.val.get(txn, node);
        let val_len = val.dim().flat_len();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(val_len).flatten_mut().copy(val.as_view().flatten(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _store_grad(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.grad.var()) {
      if writer.downcast_mut::<CursorBuf<Vec<f32>>>().is_some() {
        let mut grad = self.data.grad.get(txn, node);
        let grad_len = grad.dim().flat_len();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(grad_len).flatten_mut().copy(grad.as_view().flatten(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.data.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
  }

  fn _backward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_forward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_backward(&self, _txn: TxnId) {
  }

  fn _reset_clock(&self) {
    if self.clock {
      self.data.reset_clock_all();
    }
  }

  fn _set_clock(&self, clk: usize) {
    if self.clock {
      self.data.set_clock_all(clk);
    }
  }
}

impl ArrayOp<DeviceArray4d<f32>> for ArraySrc<DeviceArray4d<f32>> {
  fn data(&self) -> ArrayData<DeviceArray4d<f32>> {
    self.data.clone()
  }
}

impl AutodiffOp for ArraySrc<DeviceArray4d<f32>> {
  fn _load_val(&self, txn: TxnId, vars: &mut VarSet, reader: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if self.data.val.overwrite(txn, node) {
        if reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
          let mut val = self.data.val.get_mut(txn, node);
          let val_len = val.dim().flat_len();
          let reader = reader.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
          val.as_view_mut().flatten_mut().copy(reader.read_buf(val_len).flatten(), DeviceStream::implicit().conn());
        } else {
          unimplemented!();
        }
      }
    }
  }

  fn _store_val(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.val.var()) {
      if writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().is_some() {
        let mut val = self.data.val.get(txn, node);
        let val_len = val.dim().flat_len();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(val_len).flatten_mut().copy(val.as_view().flatten(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _store_grad(&self, txn: TxnId, vars: &mut VarSet, writer: &mut Any) {
    let node = self._id();
    if vars.contains(&self.data.grad.var()) {
      if writer.downcast_mut::<CursorBuf<Vec<f32>>>().is_some() {
        let mut grad = self.data.grad.get(txn, node);
        let grad_len = grad.dim().flat_len();
        let writer = writer.downcast_mut::<CursorBuf<DeviceMem<f32>>>().unwrap();
        writer.write_buf(grad_len).flatten_mut().copy(grad.as_view().flatten(), DeviceStream::implicit().conn());
      } else {
        unimplemented!();
      }
    }
  }

  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.data.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
  }

  fn _backward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_forward(&self, _txn: TxnId, _gauss_newton: bool) {
  }

  fn _r_backward(&self, _txn: TxnId) {
  }

  fn _reset_clock(&self) {
    if self.clock {
      self.data.reset_clock_all();
    }
  }

  fn _set_clock(&self, clk: usize) {
    if self.clock {
      self.data.set_clock_all(clk);
    }
  }
}

impl<F> AutodiffOp for InitializeOp<DeviceArray1d<f32>, Rc<F>> where F: Fn(Rc<RefCell<ChaChaRng>>, &mut DeviceArray1d<f32>) {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    // Do nothing, `data` belongs to `x`.
  }

  fn _init(&self, txn: TxnId, seed_rng: Rc<RefCell<ChaChaRng>>) {
    let node = self._id();
    if self.data.val.overwrite(txn, node) {
      (self.kernel)(seed_rng, &mut *self.data.val.get_excl(txn, node));
    }
  }

  fn _forward(&self, txn: TxnId) {
  }

  fn _backward(&self, _txn: TxnId, _gauss_newton: bool) {
  }
}

impl<Op> SpecialMapExt</*f32,*/ DeviceArray1d<f32>> for Rc<Op> where Op: 'static + ArrayOp<DeviceArray1d<f32>> {
  fn rect(&self) -> Rc<MapOp<DeviceArray1d<f32>, RectMapKernel>> {
    let clk_horizon = self.data().horizon();
    MapOp::new(RectMapKernel, self.clone(), clk_horizon, {
      let x = self.data();
      Rc::new(move |txn, node| {
        let dim = x.val.get(txn, node).dim();
        DeviceArray1d::zeros(dim, DeviceStream::implicit().conn())
      })
    })
  }
}

impl AutodiffOp for MapOp<DeviceArray1d<f32>, RectMapKernel> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let x_dim = self.x.val.get(txn, node).dim();
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      unsafe { arraydiff_cuda_kernel_rect_fwd_f32(
          x_dim,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
          conn.raw_stream().ptr,
      ) };
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let y_dim = self.y.grad.get(txn, node).dim();
      // TODO: post/wait.
      unsafe { arraydiff_cuda_kernel_rect_bwd_f32(
          y_dim,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.grad.get(txn, node).as_view().as_ptr(),
          self.x.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          DeviceStream::implicit().conn().raw_stream().ptr,
      ) };
    }
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.y.r_val.overwrite(txn, node) {
      let x_dim = self.x.r_val.get(txn, node).dim();
      // TODO: post/wait.
      unsafe { arraydiff_cuda_kernel_rect_bwd_f32(
          x_dim,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.x.r_val.get(txn, node).as_view().as_ptr(),
          self.y.r_val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
          DeviceStream::implicit().conn().raw_stream().ptr,
      ) };
    }
  }

  fn _r_backward(&self, txn: TxnId) {
    let node = self._id();
    if self.x.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let y_dim = self.y.r_grad.get(txn, node).dim();
      // TODO: post/wait.
      unsafe { arraydiff_cuda_kernel_rect_bwd_f32(
          y_dim,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.r_grad.get(txn, node).as_view().as_ptr(),
          self.x.r_grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          DeviceStream::implicit().conn().raw_stream().ptr,
      ) };
    }
  }

  fn _backward2(&self, txn: TxnId) {
    let node = self._id();
    if self.x.grad2.accumulate(txn, node, |grad2| grad2.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let y_dim = self.y.grad2.get(txn, node).dim();
      // TODO: post/wait.
      unsafe { arraydiff_cuda_kernel_rect_bwd_f32(
          y_dim,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.grad2.get(txn, node).as_view().as_ptr(),
          self.x.grad2.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          DeviceStream::implicit().conn().raw_stream().ptr,
      ) };
    }
  }
}

impl<Op> CastExt<DeviceBatchArray3d<u8>, DeviceBatchArray3d<f32>> for Rc<Op> where Op: 'static + ArrayOp<DeviceBatchArray3d<u8>> {
  fn cast(&self) -> Rc<TransformOp<DeviceBatchArray3d<u8>, DeviceBatchArray3d<f32>, CastTransform>> {
    let clk_horizon = self.data().horizon();
    TransformOp::new(self.clone(), CastTransform, clk_horizon, {
      let x = self.data();
      Rc::new(move |txn, node| {
        let dim = x.val.get(txn, node).dim();
        let batch_cap = x.val.get(txn, node).batch_capacity();
        DeviceBatchArray3d::zeros(dim, batch_cap, DeviceStream::implicit().conn())
      })
    })
  }
}

impl ArrayOp<DeviceBatchArray3d<f32>> for TransformOp<DeviceBatchArray3d<u8>, DeviceBatchArray3d<f32>, CastTransform> {
  fn data(&self) -> ArrayData<DeviceBatchArray3d<f32>> {
    self.y.clone()
  }
}

impl AutodiffOp for TransformOp<DeviceBatchArray3d<u8>, DeviceBatchArray3d<f32>, CastTransform> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let x_dim = self.x.val.get(txn, node).dim();
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      unsafe { arraydiff_cuda_kernel_cast_u8_to_f32(
          x_dim.flat_len(),
          self.x.val.get(txn, node).as_view().flatten().as_ptr(),
          self.y.val.get_excl(txn, node).as_view_mut().flatten_mut().as_mut_ptr(),
          conn.raw_stream().ptr,
      ) };
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    // TODO
    /*let node = self._id();
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0)) {
      self.x.grad.get_mut(txn, node).as_view_mut().flatten_mut().add(1.0, self.y.grad.get(txn, node).as_view());
    }*/
  }

  /*fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.y.r_val.overwrite(txn, node) {
      self.y.r_val.get_excl(txn, node).as_view_mut().copy(self.x.r_val.get(txn, node).as_view().flatten());
    }
  }

  fn _r_backward(&self, txn: TxnId) {
    let node = self._id();
    if self.x.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0)) {
      self.x.r_grad.get_mut(txn, node).as_view_mut().flatten_mut().add(1.0, self.y.r_grad.get(txn, node).as_view());
    }
  }*/
}

impl<Op> ReifyExt<(usize, usize, usize), DeviceBatchIoMem<u8>, DeviceBatchArray3d<u8>> for Rc<Op> where Op: 'static + ArrayOp<DeviceBatchIoMem<u8>> {
  fn reify(&self, dim: (usize, usize, usize)) -> Rc<TransformOp<DeviceBatchIoMem<u8>, DeviceBatchArray3d<u8>, ReifyTransform<(usize, usize, usize)>>> {
    let clk_horizon = self.data().horizon();
    TransformOp::new(self.clone(), ReifyTransform{dim: dim}, clk_horizon, {
      let x = self.data();
      Rc::new(move |txn, node| {
        // TODO: DeviceBatchIoMem has no present capacity, only a current size.
        let batch_cap = x.val.get(txn, node).batch_size();
        //let batch_cap = x.val.get(txn, node).batch_capacity();
        DeviceBatchArray3d::zeros(dim, batch_cap, DeviceStream::implicit().conn())
      })
    })
  }
}

impl ArrayOp<DeviceBatchArray3d<u8>> for TransformOp<DeviceBatchIoMem<u8>, DeviceBatchArray3d<u8>, ReifyTransform<(usize, usize, usize)>> {
  fn data(&self) -> ArrayData<DeviceBatchArray3d<u8>> {
    self.y.clone()
  }
}

impl AutodiffOp for TransformOp<DeviceBatchIoMem<u8>, DeviceBatchArray3d<u8>, ReifyTransform<(usize, usize, usize)>> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let y_dim = self.y.val.get_excl(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      for idx in 0 .. batch_sz {
        let conn = DeviceStream::implicit().conn();
        self.y.val.get_excl(txn, node).as_view_mut()
          .view_mut((0, 0, 0, idx), (y_dim.0, y_dim.1, y_dim.2, idx + 1))
          .flatten_mut()
          .copy(self.x.val.get(txn, node)[idx].as_ref().flatten(), conn);
      }
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
  }
}

impl<Op> MultiplyExt<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>> for Rc<Op> where Op: 'static + ArrayOp<DeviceArray1d<f32>> {
  fn mult(&self, x_: Rc<ArrayOp<DeviceArray1d<f32>>>) -> Rc<LinearOp<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>>> {
    let clk_horizon = x_.data().horizon();
    LinearOp::new(self.clone(), x_, None, clk_horizon, Rc::new(|_, _| unsafe { DeviceMem::<f32>::alloc(1, DeviceStream::implicit().conn()) }))
  }

  fn mult_add(&self, x_: Rc<ArrayOp<DeviceArray1d<f32>>>, b_: Rc<ArrayOp<DeviceMem<f32>>>) -> Rc<LinearOp<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>>> {
    let clk_horizon = x_.data().horizon();
    LinearOp::new(self.clone(), x_, Some(b_), clk_horizon, Rc::new(|_, _| unsafe { DeviceMem::<f32>::alloc(1, DeviceStream::implicit().conn()) }))
  }
}

impl ArrayOp<DeviceMem<f32>> for LinearOp<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>> {
  fn data(&self) -> ArrayData<DeviceMem<f32>> {
    self.y.clone()
  }
}

impl AutodiffObjective for LinearOp<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>> {
  fn _set_source(&self, txn: TxnId) {
    let node = self._id();
    if self.y.grad.accumulate(txn, node, |grad| grad.as_mut().set_constant(1.0, DeviceStream::implicit().conn())) {
    } else {
      // TODO
      //assert_eq!(1.0, *self.y.grad.get_mut(txn, node));
    }
  }
}

impl AutodiffOp for LinearOp<DeviceArray1d<f32>, DeviceArray1d<f32>, DeviceMem<f32>, DeviceMem<f32>> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.a_._push(epoch, apply);
      self.x_._push(epoch, apply);
      if let Some(ref b_) = self.b_ {
        b_._push(epoch, apply);
      }
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      if let Some(ref b_) = self.b_ {
        b_._pop(epoch, apply);
      }
      self.x_._pop(epoch, apply);
      self.a_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      self.y.val.get_excl(txn, node).as_mut().inner_prod(self.a.val.get(txn, node).as_view(), self.x.val.get(txn, node).as_view(), DeviceStream::implicit().conn());
      if let Some(ref b) = self.b {
        self.y.val.get_excl(txn, node).as_mut().reshape_mut(1).add(1.0, b.val.get(txn, node).as_ref().reshape(1), DeviceStream::implicit().conn());
      }
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.a.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      // FIXME
      self.a.grad.get_mut(txn, node).as_view_mut().add(1.0, self.x.val.get(txn, node).as_view(), DeviceStream::implicit().conn());
      //self.a.grad.get_mut(txn, node).as_view_mut().add(*self.y.grad.get(txn, node), self.x.val.get(txn, node).as_view(), DeviceStream::implicit().conn());
    }
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      // FIXME
      self.x.grad.get_mut(txn, node).as_view_mut().add(1.0, self.a.val.get(txn, node).as_view(), DeviceStream::implicit().conn());
      //self.x.grad.get_mut(txn, node).as_view_mut().add(*self.y.grad.get(txn, node), self.a.val.get(txn, node).as_view(), DeviceStream::implicit().conn());
    }
    if let Some(ref b) = self.b {
      /*if b.grad.accumulate(txn, node, |g| *g = 0.0) {
        *b.grad.get_mut(txn, node) += *self.y.grad.get(txn, node);
      }*/
      unimplemented!();
    }
  }
}

impl<Op> ScalarLinearExt<f32, DeviceBatchArray3d<f32>> for Rc<Op> where Op: 'static + ArrayOp<f32> {
  fn scale(&self, x_: Rc<ArrayOp<DeviceBatchArray3d<f32>>>) -> Rc<ElemLinearOp<f32, DeviceBatchArray3d<f32>, ScaleElemKernel>> {
    let clk_horizon = x_.data().horizon();
    ElemLinearOp::new(self.clone(), x_.clone(), None, ScaleElemKernel, clk_horizon, {
      let x = x_.data();
      Rc::new(move |txn, node| {
        let dim = x.val.get(txn, node).dim();
        let batch_cap = x.val.get(txn, node).batch_capacity();
        DeviceBatchArray3d::zeros(dim, batch_cap, DeviceStream::implicit().conn())
      })
    })
  }
}

impl AutodiffOp for ElemLinearOp<f32, DeviceBatchArray3d<f32>, ScaleElemKernel> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.a_._push(epoch, apply);
      self.x_._push(epoch, apply);
      if let Some(ref b_) = self.b_ {
        b_._push(epoch, apply);
      }
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      if let Some(ref b_) = self.b_ {
        b_._pop(epoch, apply);
      }
      self.x_._pop(epoch, apply);
      self.a_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    // TODO
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let batch_sz = self.x.val.get(txn, node).batch_size();
      if let Some(ref b) = self.b {
      }
    }
    unimplemented!();
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    // TODO
    unimplemented!();
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    // TODO
    unimplemented!();
  }

  fn _r_backward(&self, txn: TxnId) {
    // TODO
    unimplemented!();
  }

  fn _backward2(&self, txn: TxnId) {
    // TODO
    unimplemented!();
  }
}

pub struct CudnnConvKernelSize {
  batch_sz:     usize,
  scratch_req:  usize,
  fwd:      CudnnConvFwdOp,
  bwd_w:    CudnnConvBwdFilterOp,
  bwd_d:    CudnnConvBwdDataOp,
  add:      CudnnAddOp,
}

pub struct CudnnConvKernel {
  scratch_sz:   Cell<usize>,
  scratch:  RefCell<DeviceMem<u8>>,
  sizes:    RefCell<HashMap<usize, CudnnConvKernelSize>>,
}

impl<Op> ConvExt<(usize, usize), DeviceArray4d<f32>, DeviceArray1d<f32>, DeviceBatchArray3d<f32>, CudnnConvKernel> for Rc<Op> where Op: 'static + ArrayOp<DeviceArray4d<f32>> {
  fn conv(&self, shape: ConvShape<(usize, usize)>, x_: Rc<ArrayOp<DeviceBatchArray3d<f32>>>) -> Rc<ConvOp<(usize, usize), DeviceArray4d<f32>, DeviceArray1d<f32>, DeviceBatchArray3d<f32>, CudnnConvKernel>> {
    let clk_horizon = x_.data().horizon();
    // TODO: the default of 4096 might need to be decreased.
    let kernel = CudnnConvKernel{
      scratch_sz:   Cell::new(4096),
      scratch:      RefCell::new(DeviceMem::zeros(4096, DeviceStream::implicit().conn())),
      sizes:        RefCell::new(HashMap::new()),
    };
    ConvOp::new(shape, self.clone(), x_.clone(), None, kernel, clk_horizon, {
      let x = x_.data();
      Rc::new(move |txn, node| {
        let dim = x.val.get(txn, node).dim();
        let batch_cap = x.val.get(txn, node).batch_capacity();
        DeviceBatchArray3d::zeros(dim, batch_cap, DeviceStream::implicit().conn())
      })
    })
  }

  fn conv_add(&self, shape: ConvShape<(usize, usize)>, x_: Rc<ArrayOp<DeviceBatchArray3d<f32>>>, b_: Rc<ArrayOp<DeviceArray1d<f32>>>) -> Rc<ConvOp<(usize, usize), DeviceArray4d<f32>, DeviceArray1d<f32>, DeviceBatchArray3d<f32>, CudnnConvKernel>> {
    unimplemented!();
  }
}

impl ArrayOp<DeviceBatchArray3d<f32>> for ConvOp<(usize, usize), DeviceArray4d<f32>, DeviceArray1d<f32>, DeviceBatchArray3d<f32>, CudnnConvKernel> {
  fn data(&self) -> ArrayData<DeviceBatchArray3d<f32>> {
    self.y.clone()
  }
}

impl AutodiffOp for ConvOp<(usize, usize), DeviceArray4d<f32>, DeviceArray1d<f32>, DeviceBatchArray3d<f32>, CudnnConvKernel> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.a_._push(epoch, apply);
      self.x_._push(epoch, apply);
      if let Some(ref b_) = self.b_ {
        b_._push(epoch, apply);
      }
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      if let Some(ref b_) = self.b_ {
        b_._pop(epoch, apply);
      }
      self.x_._pop(epoch, apply);
      self.a_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let x_dim = self.x.val.get(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      let mut sizes = self.kernel.sizes.borrow_mut();
      if !sizes.contains_key(&batch_sz) {
        let mut workspace_size = 0;
        let (in_w, in_h, in_chan) = x_dim;
        let (out_w, out_h, out_chan) = self.shape.conv2d_output_dim(x_dim);
        let (kernel_w, kernel_h) = self.shape.kernel;
        let (stride_w, stride_h) = self.shape.stride;
        let (pad_w, pad_h) = self.shape.zero_pad;
        let conn = DeviceStream::implicit().conn();
        let fwd = CudnnConvFwdOp::create_fastest(
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, in_chan, batch_sz).unwrap(),
            CudnnFilterDesc::<f32>::create_4d(kernel_w, kernel_h, in_chan, out_chan).unwrap(),
            CudnnConvDesc::create_2d(stride_w, stride_h, pad_w, pad_h).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, out_chan, batch_sz).unwrap(),
            &*conn.cudnn(),
        ).unwrap();
        workspace_size = max(workspace_size, fwd.work_size);
        let bwd_w = CudnnConvBwdFilterOp::create_fastest(
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, in_chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, out_chan, batch_sz).unwrap(),
            CudnnConvDesc::create_2d(stride_w, stride_h, pad_w, pad_h).unwrap(),
            CudnnFilterDesc::<f32>::create_4d(kernel_w, kernel_h, in_chan, out_chan).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, 1, 1, out_chan, 1).unwrap(),
            &*conn.cudnn(),
        ).unwrap();
        workspace_size = max(workspace_size, bwd_w.work_size);
        let bwd_d = CudnnConvBwdDataOp::create_fastest(
            CudnnFilterDesc::<f32>::create_4d(kernel_w, kernel_h, in_chan, out_chan).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, out_chan, batch_sz).unwrap(),
            CudnnConvDesc::create_2d(stride_w, stride_h, pad_w, pad_h).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, in_chan, batch_sz).unwrap(),
            &*conn.cudnn(),
        ).unwrap();
        workspace_size = max(workspace_size, bwd_d.work_size);
        let add = CudnnAddOp::new(
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, 1, 1, out_chan, 1).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, out_chan, batch_sz).unwrap(),
        );
        let conv = CudnnConvKernelSize{
          batch_sz:     batch_sz,
          scratch_req:  workspace_size,
          fwd:      fwd,
          bwd_w:    bwd_w,
          bwd_d:    bwd_d,
          add:      add,
        };
        sizes.insert(batch_sz, conv);
        if workspace_size > self.kernel.scratch_sz.get() {
          self.kernel.scratch_sz.set(workspace_size);
          *self.kernel.scratch.borrow_mut() = DeviceMem::zeros(workspace_size, conn);
        }
      }
      let conn = DeviceStream::implicit().conn();
      self.a.val.get(txn, node).as_view().wait(&conn);
      if let Some(ref b) = self.b {
        b.val.get(txn, node).as_view().wait(&conn);
      }
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      self.kernel.scratch.borrow_mut().as_ref().wait(&conn);
      let conv = sizes.get(&batch_sz).unwrap();
      unsafe { conv.fwd.forward(
          1.0,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.a.val.get(txn, node).as_view().as_ptr(),
          0.0,
          self.y.val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
          self.kernel.scratch.borrow_mut().as_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ) }.unwrap();
      if let Some(ref b) = self.b {
        unsafe { conv.add.forward(
            1.0,
            b.val.get(txn, node).as_view().as_ptr(),
            1.0,
            self.y.val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
            &*conn.cudnn(),
        ) }.unwrap();
      }
      self.a.val.get(txn, node).as_view().post(&conn);
      if let Some(ref b) = self.b {
        b.val.get(txn, node).as_view().post(&conn);
      }
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
      self.kernel.scratch.borrow_mut().as_ref().post(&conn);
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    let batch_sz = self.x.val.get(txn, node).batch_size();
    let mut sizes = self.kernel.sizes.borrow_mut();
    let conv = sizes.get(&batch_sz).unwrap();
    // TODO: wait-post.
    if self.a.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let conn = DeviceStream::implicit().conn();
      unsafe { conv.bwd_w.backward_filter(
          1.0,
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.grad.get(txn, node).as_view().as_ptr(),
          1.0,
          self.a.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          self.kernel.scratch.borrow_mut().as_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ).unwrap() };
    }
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let conn = DeviceStream::implicit().conn();
      unsafe { conv.bwd_d.backward_data(
          1.0,
          self.a.val.get(txn, node).as_view().as_ptr(),
          self.y.grad.get(txn, node).as_view().as_ptr(),
          1.0,
          self.x.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          self.kernel.scratch.borrow_mut().as_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ).unwrap() };
    }
    if let Some(ref b) = self.b {
      if b.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
        let conn = DeviceStream::implicit().conn();
        unsafe { conv.bwd_w.backward_bias(
            1.0,
            self.y.grad.get(txn, node).as_view().as_ptr(),
            1.0,
            b.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
            &*conn.cudnn(),
        ).unwrap() };
      }
    }
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.y.r_val.overwrite(txn, node) {
      unimplemented!();
    }
  }

  fn _r_backward(&self, txn: TxnId) {
    let node = self._id();
    if self.a.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
    if self.x.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }

  fn _backward2(&self, txn: TxnId) {
    let node = self._id();
    if self.a.grad2.accumulate(txn, node, |grad2| grad2.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
    if self.x.grad2.accumulate(txn, node, |grad2| grad2.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }
}

pub struct CudnnPoolKernelSize {
  batch_sz: usize,
  pooling:  CudnnPoolingOp,
}

pub struct CudnnPoolKernel {
  sizes:    RefCell<HashMap<usize, CudnnPoolKernelSize>>,
  //scratch:  RefCell<DeviceMem<u8>>,
}

impl<PoolTy, Kernel> ArrayOp<DeviceBatchArray3d<f32>> for PoolOp<PoolTy, (usize, usize), DeviceBatchArray3d<f32>, Kernel> where PoolOp<PoolTy, (usize, usize), DeviceBatchArray3d<f32>, Kernel>: AutodiffOp {
  fn data(&self) -> ArrayData<DeviceBatchArray3d<f32>> {
    self.y.clone()
  }
}

impl AutodiffOp for PoolOp<AvgPool, (usize, usize), DeviceBatchArray3d<f32>, CudnnPoolKernel> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let x_dim = self.x.val.get(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      let mut sizes = self.kernel.sizes.borrow_mut();
      if !sizes.contains_key(&batch_sz) {
        let (in_w, in_h, chan) = x_dim;
        let (out_w, out_h, _) = self.shape.conv2d_output_dim(x_dim);
        let (kern_w, kern_h) = self.shape.kernel;
        let (stride_w, stride_h) = self.shape.stride;
        let (pad_w, pad_h) = self.shape.zero_pad;
        let pooling = match CudnnPoolingOp::create_2d(
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, chan, batch_sz).unwrap(),
            kern_w,   kern_h,
            stride_w, stride_h,
            pad_w,    pad_h,
            cudnnPoolingMode_t::AverageCountIncludingPadding,
            //cudnnPoolingMode_t::Max,
        ) {
          Err(e) => panic!("failed to create CudnnPoolingOp: {:?}", e),
          Ok(pooling) => pooling,
        };
        let pool = CudnnPoolKernelSize{
          batch_sz: batch_sz,
          pooling:  pooling,
        };
        sizes.insert(batch_sz, pool);
      }
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      let pool = sizes.get(&batch_sz).unwrap();
      unsafe { pool.pooling.forward(
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ) }.unwrap();
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let x_dim = self.x.val.get(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      let mut sizes = self.kernel.sizes.borrow_mut();
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      self.y.grad.get(txn, node).as_view().wait(&conn);
      self.x.grad.get_mut(txn, node).as_view().wait(&conn);
      let pool = sizes.get(&batch_sz).unwrap();
      unsafe { pool.pooling.backward(
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.val.get_excl(txn, node).as_view().as_ptr(),
          self.y.grad.get(txn, node).as_view().as_ptr(),
          self.x.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ) }.unwrap();
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
      self.y.grad.get(txn, node).as_view().post(&conn);
      self.x.grad.get_mut(txn, node).as_view().post(&conn);
    }
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.y.r_val.overwrite(txn, node) {
      unimplemented!();
    }
  }

  fn _r_backward(&self, txn: TxnId) {
    let node = self._id();
    if self.x.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }

  fn _backward2(&self, txn: TxnId) {
    let node = self._id();
    if self.x.grad2.accumulate(txn, node, |grad2| grad2.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }
}

impl AutodiffOp for PoolOp<MaxPool, (usize, usize), DeviceBatchArray3d<f32>, CudnnPoolKernel> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.y.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    let node = self._id();
    if self.y.val.overwrite(txn, node) {
      let x_dim = self.x.val.get(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      let mut sizes = self.kernel.sizes.borrow_mut();
      if !sizes.contains_key(&batch_sz) {
        let (in_w, in_h, chan) = x_dim;
        let (out_w, out_h, _) = self.shape.conv2d_output_dim(x_dim);
        let (kern_w, kern_h) = self.shape.kernel;
        let (stride_w, stride_h) = self.shape.stride;
        let (pad_w, pad_h) = self.shape.zero_pad;
        let pooling = match CudnnPoolingOp::create_2d(
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, in_w, in_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, chan, batch_sz).unwrap(),
            CudnnTensorDesc::<f32>::create_4d(CudnnTensorLayout::NCHW, out_w, out_h, chan, batch_sz).unwrap(),
            kern_w,   kern_h,
            stride_w, stride_h,
            pad_w,    pad_h,
            cudnnPoolingMode_t::Max,
        ) {
          Err(e) => panic!("failed to create CudnnPoolingOp: {:?}", e),
          Ok(pooling) => pooling,
        };
        let pool = CudnnPoolKernelSize{
          batch_sz: batch_sz,
          pooling:  pooling,
        };
        sizes.insert(batch_sz, pool);
      }
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      let pool = sizes.get(&batch_sz).unwrap();
      unsafe { pool.pooling.forward(
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.val.get_excl(txn, node).as_view_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ) }.unwrap();
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
    }
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.x.grad.accumulate(txn, node, |grad| grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      let x_dim = self.x.val.get(txn, node).dim();
      let batch_sz = self.x.val.get(txn, node).batch_size();
      self.y.val.get_excl(txn, node).set_batch_size(batch_sz);
      let mut sizes = self.kernel.sizes.borrow_mut();
      let conn = DeviceStream::implicit().conn();
      self.x.val.get(txn, node).as_view().wait(&conn);
      self.y.val.get_excl(txn, node).as_view().wait(&conn);
      self.y.grad.get(txn, node).as_view().wait(&conn);
      self.x.grad.get_mut(txn, node).as_view().wait(&conn);
      let pool = sizes.get(&batch_sz).unwrap();
      unsafe { pool.pooling.backward(
          self.x.val.get(txn, node).as_view().as_ptr(),
          self.y.val.get_excl(txn, node).as_view().as_ptr(),
          self.y.grad.get(txn, node).as_view().as_ptr(),
          self.x.grad.get_mut(txn, node).as_view_mut().as_mut_ptr(),
          &*conn.cudnn(),
      ) }.unwrap();
      self.x.val.get(txn, node).as_view().post(&conn);
      self.y.val.get_excl(txn, node).as_view().post(&conn);
      self.y.grad.get(txn, node).as_view().post(&conn);
      self.x.grad.get_mut(txn, node).as_view().post(&conn);
    }
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    let node = self._id();
    if self.y.r_val.overwrite(txn, node) {
      unimplemented!();
    }
  }

  fn _r_backward(&self, txn: TxnId) {
    let node = self._id();
    if self.x.r_grad.accumulate(txn, node, |r_grad| r_grad.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }

  fn _backward2(&self, txn: TxnId) {
    let node = self._id();
    if self.x.grad2.accumulate(txn, node, |grad2| grad2.as_view_mut().set_constant(0.0, DeviceStream::implicit().conn())) {
      unimplemented!();
    }
  }
}

impl<Op> SoftmaxNLLLossExt<Op, DeviceBatchArray1d<f32>, Batch<u32>, Batch<f32>> for Rc<Op> where Op: ArrayOp<DeviceBatchArray1d<f32>> {
  fn softmax_nll_loss(x_: Rc<Op>, target_: Rc<ArrayOp<Batch<u32>>>) -> Rc<SoftmaxLoss<DeviceBatchArray1d<f32>, Batch<u32>, Batch<f32>, NLLLossLink>> {
    unimplemented!();
  }
}

impl ArrayOp<Batch<f32>> for SoftmaxLoss<DeviceBatchArray1d<f32>, Batch<u32>, Batch<f32>, NLLLossLink> {
  fn data(&self) -> ArrayData<Batch<f32>> {
    self.loss.clone()
  }
}

impl AutodiffOp for SoftmaxLoss<DeviceBatchArray1d<f32>, Batch<u32>, Batch<f32>, NLLLossLink> {
  fn _id(&self) -> NodeId {
    self.node_id
  }

  fn _push(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if 1 == self.stack.push(epoch) {
      self.x_._push(epoch, apply);
      if let Some(ref target_) = self.target_ {
        target_._push(epoch, apply);
      }
      apply(self);
    }
  }

  fn _pop(&self, epoch: Epoch, apply: &mut FnMut(&AutodiffOp)) {
    if self.stack.degree(epoch) == self.stack.pop(epoch) {
      apply(self);
      if let Some(ref target_) = self.target_ {
        target_._pop(epoch, apply);
      }
      self.x_._pop(epoch, apply);
    }
  }

  fn _rollover(&self, txn: TxnId, vars: &mut VarSet) {
    self.loss.rollover_all(txn, vars);
  }

  fn _forward(&self, txn: TxnId) {
    unimplemented!();
  }

  fn _backward(&self, txn: TxnId, _gauss_newton: bool) {
    unimplemented!();
  }

  fn _r_forward(&self, txn: TxnId, _gauss_newton: bool) {
    unimplemented!();
  }

  fn _r_backward(&self, txn: TxnId) {
    unimplemented!();
  }
}
