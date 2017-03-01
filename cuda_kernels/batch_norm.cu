#include <cuda_runtime_api.h>
#include <stdint.h>

#define OFFSET_BANK(idx) ({ __typeof__ (idx) _idx = idx; ((_idx) + ((_idx) / 32)); })

__global__ void conv_diag_affine_white_var_fwd_batch_kernel(
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    const float *mean,
    const float *var,
    float epsilon,
    float *y)
{
  uint32_t idx = threadIdx.x + blockIdx.x * blockDim.x;
  uint32_t u = idx % spatial_dim;
  uint32_t c = (idx / spatial_dim) % chan_dim;
  uint32_t batch_idx = idx / (spatial_dim * chan_dim);
  if (u < spatial_dim && c < chan_dim && batch_idx < batch_sz) {
    float m = mean[c];
    float v = var[c];
    float y_i = (x[idx] - m) * rsqrtf(v + epsilon);
    y[idx] = y_i;
  }
}

extern "C" void arraydiff_cuda_kernel_conv_normalize_fwd_f32(
    size_t spatial_dim,
    size_t chan_dim,
    size_t batch_sz,
    const float *x,
    const float *mean,
    const float *var,
    float epsilon,
    float *y,
    cudaStream_t stream)
{
  uint32_t n = spatial_dim * chan_dim * batch_sz;
  conv_diag_affine_white_var_fwd_batch_kernel<<<(n+1024-1)/1024, 1024, 0, stream>>>(
      spatial_dim, chan_dim, batch_sz, x, mean, var, epsilon, y);
}

__global__ void conv_normalize_var_bwd_nonatomic_f32_kernel(
    uint32_t round_offset,
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    const float *mean,
    const float *var,
    const float *y_grad,
    float epsilon,
    float *var_grad)
{
  __shared__ float cache[1024+32];
  uint32_t block_dim = min(blockDim.x, spatial_dim * batch_sz - round_offset);
  uint32_t round_idx = round_offset + threadIdx.x;
  uint32_t spatial_idx = round_idx % spatial_dim;
  uint32_t batch_idx = round_idx / spatial_dim;
  uint32_t chan_idx = blockIdx.x;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    uint32_t idx = spatial_idx + spatial_dim * (chan_idx + chan_dim * batch_idx);
    float v = var[chan_idx];
    cache[OFFSET_BANK(threadIdx.x)] = -0.5f * y_grad[idx] * (x[idx] - mean[chan_idx]) * rsqrtf(v + epsilon) / (v + epsilon);
  } else {
    cache[OFFSET_BANK(threadIdx.x)] = 0.0f;
  }
  __syncthreads();
  for (uint32_t s = 1; s < blockDim.x; s *= 2) {
    if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
      if ((threadIdx.x & (2 * s - 1)) == 0 && (threadIdx.x + s) < block_dim) {
        cache[OFFSET_BANK(threadIdx.x)] += cache[OFFSET_BANK(threadIdx.x + s)];
      }
    }
    __syncthreads();
  }
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    if (threadIdx.x == 0) {
      var_grad[chan_idx] += cache[0];
    }
  }
}

__global__ void conv_normalize_mean_bwd_nonatomic_f32_kernel(
    uint32_t round_offset,
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    const float *mean,
    const float *var,
    const float *var_grad,
    const float *y_grad,
    float epsilon,
    float *mean_grad)
{
  __shared__ float cache[1024+32];
  uint32_t block_dim = min(blockDim.x, spatial_dim * batch_sz - round_offset);
  uint32_t round_idx = round_offset + threadIdx.x;
  uint32_t spatial_idx = round_idx % spatial_dim;
  uint32_t batch_idx = round_idx / spatial_dim;
  uint32_t chan_idx = blockIdx.x;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    uint32_t idx = spatial_idx + spatial_dim * (chan_idx + chan_dim * batch_idx);
    cache[OFFSET_BANK(threadIdx.x)] = -(y_grad[idx] * rsqrtf(var[chan_idx] + epsilon) + 2.0f * var_grad[chan_idx] * (x[idx] - mean[chan_idx]) / ((float)(spatial_dim * (batch_sz - 1))));
  } else {
    cache[OFFSET_BANK(threadIdx.x)] = 0.0f;
  }
  __syncthreads();
  for (uint32_t s = 1; s < blockDim.x; s *= 2) {
    if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
      if ((threadIdx.x & (2 * s - 1)) == 0 && (threadIdx.x + s) < block_dim) {
        cache[OFFSET_BANK(threadIdx.x)] += cache[OFFSET_BANK(threadIdx.x + s)];
      }
    }
    __syncthreads();
  }
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    if (threadIdx.x == 0) {
      mean_grad[chan_idx] += cache[0];
    }
  }
}

__global__ void conv_normalize_x_bwd_nonatomic_f32_kernel(
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *var,
    const float *y_grad,
    float epsilon,
    float *x_grad)
{
  uint32_t idx = threadIdx.x + blockDim.x * blockIdx.x;
  uint32_t spatial_idx = idx % spatial_dim;
  uint32_t chan_idx = (idx / spatial_dim) % chan_dim;
  uint32_t batch_idx = (idx / spatial_dim) / chan_dim;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    x_grad[idx] += y_grad[idx] * rsqrtf(var[chan_idx] + epsilon);
  }
}

__global__ void conv_batch_mean_fwd_nonatomic_f32_kernel(
    uint32_t round_offset,
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    float *mean)
{
  __shared__ float cache[1024+32];
  uint32_t block_dim = min(blockDim.x, spatial_dim * batch_sz - round_offset);
  uint32_t round_idx = round_offset + threadIdx.x;
  uint32_t spatial_idx = round_idx % spatial_dim;
  uint32_t batch_idx = round_idx / spatial_dim;
  uint32_t chan_idx = blockIdx.x;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    uint32_t idx = spatial_idx + spatial_dim * (chan_idx + chan_dim * batch_idx);
    cache[OFFSET_BANK(threadIdx.x)] = x[idx];
  } else {
    cache[OFFSET_BANK(threadIdx.x)] = 0.0f;
  }
  __syncthreads();
  for (uint32_t s = 1; s < blockDim.x; s *= 2) {
    if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
      if ((threadIdx.x & (2 * s - 1)) == 0 && (threadIdx.x + s) < block_dim) {
        cache[OFFSET_BANK(threadIdx.x)] += cache[OFFSET_BANK(threadIdx.x + s)];
      }
    }
    __syncthreads();
  }
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    if (threadIdx.x == 0) {
      mean[chan_idx] += cache[0] / ((float)(spatial_dim * batch_sz));
    }
  }
}

extern "C" void arraydiff_cuda_kernel_conv_batch_mean_fwd_nonatomic_f32(
    size_t spatial_dim,
    size_t chan_dim,
    size_t batch_sz,
    const float *x,
    float *mean,
    cudaStream_t stream)
{
  // XXX: `mean` should be zeroed.
  uint32_t num_rounds = (spatial_dim * batch_sz + 1024-1) / 1024;
  uint32_t num_blocks = chan_dim;
  for (uint32_t round = 0; round < num_rounds; round++) {
    conv_batch_mean_fwd_nonatomic_f32_kernel<<<num_blocks, 1024, 0, stream>>>(
        round * 1024, spatial_dim, chan_dim, batch_sz, x, mean);
  }
}

__global__ void conv_batch_var_fwd_nonatomic_f32_kernel(
    uint32_t round_offset,
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    const float *mean,
    float *var)
{
  __shared__ float cache[1024+32];
  uint32_t block_dim = min(blockDim.x, spatial_dim * batch_sz - round_offset);
  uint32_t round_idx = round_offset + threadIdx.x;
  uint32_t spatial_idx = round_idx % spatial_dim;
  uint32_t batch_idx = round_idx / spatial_dim;
  uint32_t chan_idx = blockIdx.x;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    uint32_t idx = spatial_idx + spatial_dim * (chan_idx + chan_dim * batch_idx);
    float residual = x[idx] - mean[chan_idx];
    cache[OFFSET_BANK(threadIdx.x)] = residual * residual;
  } else {
    cache[OFFSET_BANK(threadIdx.x)] = 0.0f;
  }
  __syncthreads();
  for (uint32_t s = 1; s < blockDim.x; s *= 2) {
    if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
      if ((threadIdx.x & (2 * s - 1)) == 0 && (threadIdx.x + s) < block_dim) {
        cache[OFFSET_BANK(threadIdx.x)] += cache[OFFSET_BANK(threadIdx.x + s)];
      }
    }
    __syncthreads();
  }
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    if (threadIdx.x == 0) {
      var[chan_idx] += cache[0] / ((float)(spatial_dim * (batch_sz - 1)));
    }
  }
}

extern "C" void arraydiff_cuda_kernel_conv_batch_var_fwd_nonatomic_f32(
    size_t spatial_dim,
    size_t chan_dim,
    size_t batch_sz,
    const float *x,
    const float *mean,
    float *var,
    cudaStream_t stream)
{
  // XXX: `var` should be zeroed.
  uint32_t num_rounds = (spatial_dim * batch_sz + 1024-1) / 1024;
  uint32_t num_blocks = chan_dim;
  for (uint32_t round = 0; round < num_rounds; round++) {
    conv_batch_var_fwd_nonatomic_f32_kernel<<<num_blocks, 1024, 0, stream>>>(
        round * 1024, spatial_dim, chan_dim, batch_sz, x, mean, var);
  }
}

__global__ void conv_batch_stats_bwd_f32_kernel(
    uint32_t spatial_dim,
    uint32_t chan_dim,
    uint32_t batch_sz,
    const float *x,
    const float *mean,
    const float *mean_grad,
    const float *var_grad,
    float epsilon,
    float *x_grad)
{
  uint32_t idx = threadIdx.x + blockDim.x * blockIdx.x;
  uint32_t spatial_idx = idx % spatial_dim;
  uint32_t chan_idx = (idx / spatial_dim) % chan_dim;
  uint32_t batch_idx = (idx / spatial_dim) / chan_dim;
  if (spatial_idx < spatial_dim && chan_idx < chan_dim && batch_idx < batch_sz) {
    x_grad[idx] += mean_grad[chan_idx] / ((float)(spatial_dim * batch_sz)) + 2.0f * var_grad[chan_idx] * (x[idx] - mean[chan_idx]) / ((float)(spatial_dim * (batch_sz - 1)));;
  }
}