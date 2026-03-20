//! GPU buffer utilities.

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, Buffer, BufferUsages, Device, Queue};

/// A dynamically growing GPU buffer for vertex/instance data.
pub struct DynamicBuffer<T: Pod + Zeroable> {
    buffer: Buffer,
    capacity: usize,
    len: usize,
    usage: BufferUsages,
    label: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Pod + Zeroable> DynamicBuffer<T> {
    /// Create a new dynamic buffer with the given initial capacity.
    pub fn new(device: &Device, label: &'static str, usage: BufferUsages, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (capacity * std::mem::size_of::<T>()) as u64,
            usage: usage | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            capacity,
            len: 0,
            usage,
            label,
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a vertex buffer.
    pub fn vertex(device: &Device, label: &'static str, capacity: usize) -> Self {
        Self::new(device, label, BufferUsages::VERTEX, capacity)
    }

    /// Create an index buffer.
    pub fn index(device: &Device, label: &'static str, capacity: usize) -> Self {
        Self::new(device, label, BufferUsages::INDEX, capacity)
    }

    /// Write data to the buffer, growing if necessary.
    pub fn write(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        self.len = data.len();

        if data.is_empty() {
            return;
        }

        // Grow buffer if needed
        if data.len() > self.capacity {
            self.capacity = (data.len() * 2).next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: (self.capacity * std::mem::size_of::<T>()) as u64,
                usage: self.usage | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    /// Get the underlying wgpu buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get the number of elements currently in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A static GPU buffer initialized once.
pub struct StaticBuffer<T: Pod + Zeroable> {
    buffer: Buffer,
    len: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Pod + Zeroable> StaticBuffer<T> {
    /// Create a new static vertex buffer with initial data.
    pub fn vertex(device: &Device, label: &'static str, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::VERTEX,
        });

        Self {
            buffer,
            len: data.len(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new static index buffer with initial data.
    pub fn index(device: &Device, label: &'static str, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::INDEX,
        });

        Self {
            buffer,
            len: data.len(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the underlying wgpu buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Resources for instanced quad rendering (shared vertex/index buffers).
pub struct QuadBuffers {
    pub vertices: StaticBuffer<crate::vertex::Vertex2D>,
    pub indices: StaticBuffer<u16>,
}

impl QuadBuffers {
    pub fn new(device: &Device) -> Self {
        use crate::vertex::Vertex2D;

        Self {
            vertices: StaticBuffer::vertex(device, "quad_vertices", &Vertex2D::UNIT_QUAD),
            indices: StaticBuffer::index(device, "quad_indices", &Vertex2D::UNIT_QUAD_INDICES),
        }
    }
}
