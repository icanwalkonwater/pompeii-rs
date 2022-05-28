use std::{array::from_ref, collections::HashMap, hash::Hash};

use ash::vk;
use log::debug;

use crate::{acceleration_structure::Tlas, errors::Result, PompeiiRenderer};

#[repr(u8)]
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum RtBindings {
    Tlas = 0,
    Image = 1,
}

impl Into<u32> for RtBindings {
    fn into(self) -> u32 {
        self as _
    }
}

#[derive(Debug, Clone)]
pub struct DescriptorSetHandle {
    pub handle: vk::DescriptorSet,
    pub layout: vk::DescriptorSetLayout,
}

impl PompeiiRenderer {
    pub fn create_descriptor_set_rt(
        &self,
        tlas: &Tlas,
        /* output_image: Oui */
    ) -> Result<DescriptorSetHandle> {
        let tlas = tlas.0.handle;

        let (layout, handle) = DescriptorSetBuilder::new()
            .add_binding(
                RtBindings::Tlas,
                vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                1,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .add_binding(
                RtBindings::Image,
                vk::DescriptorType::STORAGE_IMAGE,
                1,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .write_binding_acceleration_structure(RtBindings::Tlas, 0, from_ref(&tlas))
            .build(self)?;

        Ok(DescriptorSetHandle { handle, layout })
    }
}

struct DescriptorSetBuilder<'a, T: Into<u32> + Hash + Eq + Copy> {
    bindings: HashMap<T, vk::DescriptorSetLayoutBindingBuilder<'a>>,
    pool_sizes: HashMap<vk::DescriptorType, vk::DescriptorPoolSize>,
    writes: Vec<vk::WriteDescriptorSetBuilder<'a>>,
    writes_as: Vec<(
        vk::WriteDescriptorSetBuilder<'a>,
        vk::WriteDescriptorSetAccelerationStructureKHRBuilder<'a>,
    )>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: Into<u32> + Hash + Eq + Copy> DescriptorSetBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            bindings: Default::default(),
            pool_sizes: Default::default(),
            writes: Default::default(),
            writes_as: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub fn add_binding(
        mut self,
        tag: T,
        ty: vk::DescriptorType,
        count: u32,
        stage_flags: vk::ShaderStageFlags,
    ) -> Self {
        debug_assert!(!self.bindings.contains_key(&tag));
        let binding = T::into(tag);
        self.bindings.insert(
            tag,
            vk::DescriptorSetLayoutBinding::builder()
                .binding(binding)
                .descriptor_type(ty)
                .descriptor_count(count)
                .stage_flags(stage_flags),
        );

        let size = self.pool_sizes.entry(ty).or_insert_with(|| {
            vk::DescriptorPoolSize::builder()
                .ty(ty)
                .descriptor_count(0)
                .build()
        });
        size.descriptor_count += 1;

        self
    }

    pub fn write_binding_buffer(
        mut self,
        tag: T,
        offset: u32,
        buffers: &'a [vk::DescriptorBufferInfo],
    ) -> Self {
        self.check_binding(tag, offset, buffers.len() as _, |ty| {
            matches!(
                ty,
                vk::DescriptorType::UNIFORM_BUFFER
                    | vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC
                    | vk::DescriptorType::STORAGE_BUFFER
                    | vk::DescriptorType::STORAGE_BUFFER_DYNAMIC
            )
        });

        let binding_i = T::into(tag);
        let binding = &self.bindings[&tag];
        self.writes.push(
            vk::WriteDescriptorSet::builder()
                .descriptor_type(binding.descriptor_type)
                // Not available yet
                // .dst_set()
                .dst_binding(binding_i)
                .dst_array_element(offset)
                .buffer_info(buffers),
        );

        self
    }

    pub fn write_binding_image(
        mut self,
        tag: T,
        offset: u32,
        images: &'a [vk::DescriptorImageInfo],
    ) -> Self {
        self.check_binding(tag, offset, images.len() as _, |ty| {
            matches!(
                ty,
                vk::DescriptorType::SAMPLER
                    | vk::DescriptorType::COMBINED_IMAGE_SAMPLER
                    | vk::DescriptorType::SAMPLED_IMAGE
                    | vk::DescriptorType::STORAGE_IMAGE
                    | vk::DescriptorType::INPUT_ATTACHMENT
            )
        });

        let binding_i = T::into(tag);
        let binding = &self.bindings[&tag];
        self.writes.push(
            vk::WriteDescriptorSet::builder()
                .descriptor_type(binding.descriptor_type)
                // Not available yet
                // .dst_set()
                .dst_binding(binding_i)
                .dst_array_element(offset)
                .image_info(images),
        );

        self
    }

    pub fn write_binding_acceleration_structure(
        mut self,
        tag: T,
        offset: u32,
        tlases: &'a [vk::AccelerationStructureKHR],
    ) -> Self {
        self.check_binding(tag, offset, tlases.len() as _, |ty| {
            matches!(ty, vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
        });

        let binding_i = T::into(tag);
        let binding = &self.bindings[&tag];

        self.writes_as.push((
            vk::WriteDescriptorSet::builder()
                .descriptor_type(binding.descriptor_type)
                // .dst_set()
                .dst_binding(binding_i)
                .dst_array_element(offset),
            vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                .acceleration_structures(tlases),
        ));

        self
    }

    #[inline(always)]
    fn check_binding(
        &self,
        tag: T,
        offset: u32,
        size: u32,
        matches: fn(vk::DescriptorType) -> bool,
    ) {
        if cfg!(debug_assertions) {
            debug_assert!(self.bindings.contains_key(&tag));

            let binding = &self.bindings[&tag];
            debug_assert_ne!(binding.descriptor_count, 0);
            debug_assert!(offset + size <= binding.descriptor_count);

            debug_assert!(matches(binding.descriptor_type));
        }
    }

    pub fn build(
        self,
        renderer: &PompeiiRenderer,
    ) -> Result<(vk::DescriptorSetLayout, vk::DescriptorSet)> {
        // Create pool
        let pool = unsafe {
            let pool_sizes = self.pool_sizes.into_values().collect::<Vec<_>>();
            renderer.device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::empty())
                    .max_sets(1)
                    .pool_sizes(&pool_sizes),
                None,
            )?
        };

        // Create set layout
        let layout = unsafe {
            let bindings = self
                .bindings
                .into_values()
                .map(|b| b.build())
                .collect::<Vec<_>>();
            renderer.device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::empty())
                    .bindings(&bindings),
                None,
            )?
        };

        // Finally, allocate set from pool
        let set = unsafe {
            renderer.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(from_ref(&layout)),
            )?[0]
        };

        // Complete writes
        let mut writes = self
            .writes
            .into_iter()
            .map(|write| write.dst_set(set).build())
            .collect::<Vec<_>>();

        // Append acceleration structure writes
        let (writes_as_main, mut writes_as_next) =
            self.writes_as.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
        for (mut main, tlas) in writes_as_main.into_iter().zip(writes_as_next.iter_mut()) {
            main.descriptor_count = tlas.acceleration_structure_count;
            writes.push(main.dst_set(set).push_next(tlas).build());
        }

        // Actually write
        unsafe {
            renderer.device.update_descriptor_sets(&writes, &[]);
        }

        renderer
            .main_deletion_queue
            .lock()
            .push(Box::new(move |renderer| unsafe {
                debug!("Destroy RT descriptor set");
                renderer.device.destroy_descriptor_set_layout(layout, None);
                renderer.device.destroy_descriptor_pool(pool, None);
                Ok(())
            }));

        Ok((layout, set))
    }
}
