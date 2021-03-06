/*
Copyright 2017 the arraydiff authors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

#[cfg(feature = "cuda")] use cuda::ffi::runtime::{cudaStream_t};
//use libc::*;

#[link(name = "arraydiff_kernels", kind = "static")]
extern "C" {
  // Special map functions.
  pub fn arraydiff_kernel_rect_fwd_f32(dim: usize, x: *const f32, y: *mut f32);
  pub fn arraydiff_kernel_rect_bwd_f32(dim: usize, x: *const f32, dy: *const f32, dx: *mut f32);
  pub fn arraydiff_kernel_logistic_fwd_f32(dim: usize, x: *const f32, y: *mut f32);
  pub fn arraydiff_kernel_logistic_bwd_f32(dim: usize, x: *const f32, dy: *const f32, dx: *mut f32);
  pub fn arraydiff_kernel_logistic_rbwd_f32(dim: usize, x: *const f32, r_x: *const f32, dy: *const f32, r_dy: *const f32, r_dx: *mut f32);
  pub fn arraydiff_kernel_logistic_bwd2_f32(dim: usize, x: *const f32, dy: *const f32, dy2: *const f32, dx2: *mut f32);
  pub fn arraydiff_kernel_tanh_fwd_f32(dim: usize, x: *const f32, y: *mut f32);
  pub fn arraydiff_kernel_tanh_bwd_f32(dim: usize, x: *const f32, dy: *const f32, dx: *mut f32);
  pub fn arraydiff_kernel_tanh_rbwd_f32(dim: usize, x: *const f32, r_x: *const f32, dy: *const f32, r_dy: *const f32, r_dx: *mut f32);
}

#[cfg(feature = "cuda")]
#[link(name = "arraydiff_cuda_kernels", kind = "static")]
extern "C" {
  // Special map functions.
  pub fn arraydiff_cuda_kernel_rect_fwd_f32(dim: usize, x: *const f32, y: *mut f32, stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_rect_bwd_f32(dim: usize, x: *const f32, dy: *const f32, dx: *mut f32, stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_symm_unit_clip_fwd_f32(dim: usize, clip: *const f32, x: *const f32, y: *mut f32, stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_symm_unit_clip_param_bwd_nondeterministic_f32(dim: usize, clip: *const f32, x: *const f32, y: *const f32, grad: *mut f32, stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_symm_unit_clip_input_bwd_f32(dim: usize, clip: *const f32, x: *const f32, y: *const f32, dy: *mut f32, stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_cast_u8_to_f32(dim: usize, x: *const u8, y: *mut f32, stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_cast_u8x4_to_f32x4(dim: usize, x: *const u8, y: *mut f32, stream: cudaStream_t);

  /* Source: "cuda_kernels/linear.cu" */

  /* Broadcast add kernels: [a] . [an] -> [an] */
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2an_O1an_fwd_f32(
      chan_dim: usize,
      batch_sz: usize,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2an_O1an_fwdaccum_f32(
      chan_dim: usize,
      batch_sz: usize,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2an_O1an_bwd_shift_deterministic_f32(
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2an_O1an_bwd_input_f32(
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);
  /* Broadcast add kernels: [a] . [xyan] -> [xyan] */
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2xyan_O1xyan_fwd_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2xyan_O1xyan_fwdaccum_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2xyan_O1xyan_bwd_shift_deterministic_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1a_I2xyan_O1xyan_bwd_input_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);
  /* Broadcast multiply-add kernels: [a] . [a] . [xyan] -> [xyan] */
  pub fn arraydiff_cuda_kernel_bcast_mult_add_I1a_I2a_I3xyan_O1xyan_fwd_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      scale: *const f32,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_mult_add_I1a_I2a_I3xyan_O1xyan_bwd_scale_shift_deterministic_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      y_grad: *const f32,
      scale_grad: *mut f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_mult_add_I1a_I2a_I3xyan_O1xyan_bwd_input_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      scale: *const f32,
      y_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);
  /* Broadcast add kernels: [an] . [xyan] -> [xyan] */
  pub fn arraydiff_cuda_kernel_bcast_add_I1an_I2xyan_O1xyan_fwd_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      shift: *const f32,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1an_I2xyan_O1xyan_bwd_shift_deterministic_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_bcast_add_I1an_I2xyan_O1xyan_bwd_input_f32(
      prefix_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      y_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_conv_bcast_mult_add_fwd_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      scale: *const f32,
      shift: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_bcast_mult_add_param_bwd_nonatomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      scale: *const f32,
      shift: *const f32,
      y_grad: *const f32,
      scale_grad: *mut f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_bcast_mult_add_param_bwd_atomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      scale: *const f32,
      shift: *const f32,
      y_grad: *const f32,
      scale_grad: *mut f32,
      shift_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_bcast_mult_add_input_bwd_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      scale: *const f32,
      y_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_conv_normalize_fwd_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *const f32,
      epsilon: f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_normalize_var_bwd_nonatomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *const f32,
      y_grad: *const f32,
      epsilon: f32,
      var_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_normalize_var_bwd_atomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *const f32,
      y_grad: *const f32,
      epsilon: f32,
      var_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_normalize_mean_bwd_nonatomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *const f32,
      var_grad: *const f32,
      y_grad: *const f32,
      epsilon: f32,
      mean_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_normalize_mean_bwd_atomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *const f32,
      var_grad: *const f32,
      y_grad: *const f32,
      epsilon: f32,
      mean_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_normalize_input_bwd_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      var: *const f32,
      y_grad: *const f32,
      epsilon: f32,
      x_grad: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_batch_stats_mean_fwd_nonatomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_batch_stats_mean_fwd_atomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_batch_stats_var_fwd_nonatomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_batch_stats_var_fwd_atomic_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      var: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_conv_batch_stats_bwd_f32(
      spatial_dim: usize,
      chan_dim: usize,
      batch_sz: usize,
      x: *const f32,
      mean: *const f32,
      mean_grad: *const f32,
      var_grad: *const f32,
      x_grad: *mut f32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_lst_sq1_fwd_f32(
      batch_sz: usize,
      x: *const f32,
      target: *const f32,
      loss: *mut f32,
      do_clip: u32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_lst_sq_block_fwd_f32(
      block_dim: usize,
      num_blocks: usize,
      x: *const f32,
      target: *const f32,
      loss: *mut f32,
      do_clip: u32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_lst_sq_bwd_f32(
      dim: usize,
      batch_sz: usize,
      x: *const f32,
      target: *const f32,
      df: *const f32,
      dx: *mut f32,
      do_clip: u32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_max_pool_fwd_f32(
      x_w: usize, x_h: usize, chan_dim: usize, batch_sz: usize,
      y_w: usize, y_h: usize,
      kernel_w: usize, kernel_h: usize,
      stride_w: usize, stride_h: usize,
      pad_w: usize, pad_h: usize,
      x: *const f32,
      maybe_y: *mut f32,
      maybe_mask: *mut i32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_max_pool_bwd_f32(
      x_w: usize, x_h: usize, chan_dim: usize, batch_sz: usize,
      y_w: usize, y_h: usize,
      kernel_w: usize, kernel_h: usize,
      stride_w: usize, stride_h: usize,
      pad_w: usize, pad_h: usize,
      dy: *const f32,
      mask: *const i32,
      dx: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_avg_pool_fwd_f32(
      x_w: usize, x_h: usize, chan_dim: usize, batch_sz: usize,
      y_w: usize, y_h: usize,
      kernel_w: usize, kernel_h: usize,
      stride_w: usize, stride_h: usize,
      pad_w: usize, pad_h: usize,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_avg_pool_bwd_f32(
      x_w: usize, x_h: usize, chan_dim: usize, batch_sz: usize,
      y_w: usize, y_h: usize,
      kernel_w: usize, kernel_h: usize,
      stride_w: usize, stride_h: usize,
      pad_w: usize, pad_h: usize,
      dy: *const f32,
      dx: *mut f32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_blockreduce_max_argmax_f32(
      block_dim: usize,
      num_blocks: usize,
      x: *const f32,
      x_max: *mut f32,
      x_argmax: *mut u32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_blockreduce_sum_f32(
      block_dim: usize,
      num_blocks: usize,
      x: *const f32,
      x_sum: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_reduce_index_fwd_f32(
      dim: usize,
      batch_sz: usize,
      x: *const f32,
      index: *const u32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_reduce_index_bwd_f32(
      dim: usize,
      batch_sz: usize,
      dy: *const f32,
      index: *const u32,
      dx: *mut f32,
      stream: cudaStream_t);

  pub fn arraydiff_cuda_kernel_block_softmax_fwd_f32(
      block_dim: usize,
      num_blocks: usize,
      x: *const f32,
      y: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_block_softmax_tangent_fwd_f32(
      block_dim: usize,
      num_blocks: usize,
      x: *const f32,
      x_tng: *const f32,
      y: *const f32,
      y_tng: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_block_softmax_kl2_loss_fwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      t: *const f32,
      loss: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_kl2_loss_bwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      t: *const f32,
      df: *const f32,
      dx: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_block_softmax_tangent_kl2_loss_fwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      y_tng: *const f32,
      t: *const f32,
      loss_tng: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_tangent_kl2_loss_bwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y_tng: *const f32,
      df_tng: *const f32,
      dx_tng: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_lr_loss_fwd_f32(
      dim: usize,
      batch_sz: usize,
      y: *const f32,
      index: *const u32,
      t: *const f32,
      loss: *mut f32,
      lr_clip: f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_lr_loss_bwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      index: *const u32,
      t: *const f32,
      df: *const f32,
      dx: *mut f32,
      lr_clip: f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_nll_loss_fwd_f32(
      dim: usize,
      batch_sz: usize,
      y: *const f32,
      t: *const u32,
      loss: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_softmax_nll_loss_bwd_f32(
      dim: usize,
      batch_sz: usize,
      y: *const f32,
      t: *const u32,
      df: *const f32,
      dx: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_block_softmax_negentropy_loss_fwd_accumulate_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      loss: *mut f32,
      stream: cudaStream_t);
  pub fn arraydiff_cuda_kernel_block_softmax_negentropy_loss_bwd_f32(
      block_dim: usize,
      num_blocks: usize,
      y: *const f32,
      df: *const f32,
      dx: *mut f32,
      stream: cudaStream_t);
}
