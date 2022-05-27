use std::slice::from_ref;

use ash::vk;
use log::debug;

use crate::{
    alloc::VkBufferHandle,
    errors::Result,
    mesh::{Mesh, MeshIndex, MeshVertex, VertexPosNormUvF32},
    PompeiiRenderer,
};

#[derive(Debug, Clone)]
pub(crate) struct AsData {
    pub(crate) handle: vk::AccelerationStructureKHR,
    pub(crate) buffer: VkBufferHandle,
}

impl AsData {
    pub(crate) fn destroy_on_exit(&self, renderer: &PompeiiRenderer) {
        let me = self.clone();
        renderer
            .alloc_deletion_queue
            .lock()
            .push(Box::new(move |(_, ext_as), vma| unsafe {
                let me = me;
                debug!("Destroy AS");
                ext_as.destroy_acceleration_structure(me.handle, None);
                me.buffer.destroy(vma);
                Ok(())
            }))
    }
}

#[derive(Debug, Clone)]
pub struct Blas(pub(crate) AsData);

impl Blas {
    #[inline]
    pub fn destroy_on_exit(&self, renderer: &PompeiiRenderer) {
        self.0.destroy_on_exit(renderer);
    }
}

#[derive(Debug, Default)]
pub struct BlasInput {
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    build_ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
}

#[derive(Debug)]
struct AsBuildInfo<'a> {
    build_info: vk::AccelerationStructureBuildGeometryInfoKHR,
    range_info: &'a [vk::AccelerationStructureBuildRangeInfoKHR],
    size_info: vk::AccelerationStructureBuildSizesInfoKHR,
    accel: Option<(vk::AccelerationStructureKHR, VkBufferHandle)>,
}

impl PompeiiRenderer {
    pub fn create_blas<'a>(&self, meshes: impl Iterator<Item = &'a Mesh>) -> Result<Vec<Blas>> {
        let blas_inputs = meshes
            .map(|mesh| self.object_to_vk_geometry(mesh))
            .collect::<Vec<_>>();

        self.build_blas(
            blas_inputs.iter(),
            vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
        )
    }

    fn object_to_vk_geometry(&self, mesh: &Mesh) -> BlasInput {
        let vertex_address = unsafe { self.get_buffer_address(mesh.vertex_buffer.handle) };
        let index_address = unsafe { self.get_buffer_address(mesh.index_buffer.handle) };

        let mut input = BlasInput::default();

        for sub_mesh in mesh.sub_meshes.iter() {
            let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                .vertex_format(VertexPosNormUvF32::format())
                .vertex_stride(VertexPosNormUvF32::stride())
                .vertex_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: vertex_address,
                })
                .index_type(u16::index_type())
                .index_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: index_address,
                })
                .max_vertex(sub_mesh.max_vertex_index())
                .transform_data(vk::DeviceOrHostAddressConstKHR::default());

            let geometry = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: triangles.build(),
                });

            let offset = vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(sub_mesh.vert_start as _)
                .primitive_offset(sub_mesh.index_start as _)
                .primitive_count(sub_mesh.index_count as _)
                .transform_offset(0);

            // Build them because we don't hold any reference
            input.geometries.push(geometry.build());
            input.build_ranges.push(offset.build());
        }

        input.geometries.shrink_to_fit();
        input.build_ranges.shrink_to_fit();
        input
    }

    fn build_blas<'a>(
        &self,
        inputs: impl Iterator<Item = &'a BlasInput>,
        flags: vk::BuildAccelerationStructureFlagsKHR,
    ) -> Result<Vec<Blas>> {
        let mut build_infos = inputs
            .map(|input| {
                // Partial build info to just query the build sizes
                let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                    .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                    .flags(flags)
                    .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                    .geometries(&input.geometries);

                let max_primitive_counts = input
                    .build_ranges
                    .iter()
                    .map(|r| r.primitive_count)
                    .collect::<Vec<_>>();

                let size_info = unsafe {
                    self.ext_acceleration_structure
                        .get_acceleration_structure_build_sizes(
                            vk::AccelerationStructureBuildTypeKHR::DEVICE,
                            &build_info,
                            &max_primitive_counts,
                        )
                };

                AsBuildInfo {
                    build_info: build_info.build(),
                    range_info: &input.build_ranges,
                    size_info,
                    accel: None,
                }
            })
            .collect::<Vec<_>>();

        let max_scratch_size = build_infos
            .iter()
            .map(|info| info.size_info.build_scratch_size)
            .max()
            .unwrap();

        // TODO: compaction

        let scratch_buffer = self.alloc_acceleration_structure_scratch_buffer(max_scratch_size)?;
        let scratch_address = unsafe { self.get_buffer_address(scratch_buffer.handle) };

        // TODO: chunk it by pieces of 256 Mib

        // Finish to fill the build info
        for build_info in build_infos.iter_mut() {
            let size = build_info.size_info.acceleration_structure_size;

            let as_buffer = self.alloc_acceleration_structure_buffer(size)?;
            let as_handle = unsafe {
                self.ext_acceleration_structure
                    .create_acceleration_structure(
                        &vk::AccelerationStructureCreateInfoKHR::builder()
                            .buffer(as_buffer.handle)
                            .offset(0)
                            .size(size)
                            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                            .device_address(vk::DeviceAddress::default()),
                        None,
                    )?
            };

            build_info.accel = Some((as_handle, as_buffer));
            build_info.build_info.dst_acceleration_structure = as_handle;
            build_info.build_info.scratch_data = vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            };
        }

        let compute = self.queues.compute();
        let cmds = unsafe {
            self.record_one_time_command_buffer(compute.pool, |cmds| {
                self.cmd_build_blas(cmds, &build_infos)?;
                Ok(())
            })
        }?;

        unsafe {
            self.submit_and_wait(compute.queue, cmds, &[], &[], &[])?;
            self.free_buffer(scratch_buffer);
        }

        Ok(build_infos
            .into_iter()
            .map(|i| {
                let (handle, buffer) = i.accel.unwrap();
                Blas(AsData { handle, buffer })
            })
            .collect())
    }

    unsafe fn cmd_build_blas<'a>(
        &self,
        cmds: vk::CommandBuffer,
        build_infos: impl IntoIterator<Item = &'a AsBuildInfo<'a>>,
    ) -> Result<()> {
        for mut build_info in build_infos {
            self.ext_acceleration_structure
                .cmd_build_acceleration_structures(
                    cmds,
                    from_ref(&build_info.build_info),
                    from_ref(&build_info.range_info),
                );

            // For scratch buffer
            self.device.cmd_pipeline_barrier2(
                cmds,
                &vk::DependencyInfo::builder().memory_barriers(from_ref(
                    &vk::MemoryBarrier2::builder()
                        .src_stage_mask(vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
                        .src_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR)
                        .dst_stage_mask(vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
                        .dst_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR),
                )),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Tlas(pub(crate) AsData);

impl Tlas {
    #[inline]
    pub fn destroy_on_exit(&self, renderer: &PompeiiRenderer) {
        self.0.destroy_on_exit(renderer);
    }
}

impl PompeiiRenderer {
    pub fn create_tlas<'a>(&self, blases: impl Iterator<Item = &'a Blas>) -> Result<Tlas> {
        let objects = blases
            .enumerate()
            .map(|(i, blas)| vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: {
                        let mut transform = [0.0; 12];
                        // Row major 3x4 identity matrix
                        // 1 0 0 0
                        // 0 1 0 0
                        // 0 0 1 0
                        transform[0] = 1.0;
                        transform[6] = 1.0;
                        transform[10] = 1.0;
                        transform
                    },
                },
                // TODO: this index sucks ass
                instance_custom_index_and_mask: vk::Packed24_8::new(i as _, 0xff),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: unsafe { self.get_buffer_address(blas.0.buffer.handle) },
                },
            })
            .collect::<Vec<_>>();

        self.build_tlas(
            &objects,
            vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
        )
    }

    fn build_tlas(
        &self,
        instances: &[vk::AccelerationStructureInstanceKHR],
        flags: vk::BuildAccelerationStructureFlagsKHR,
    ) -> Result<Tlas> {
        let mut transfer_ctx = self.start_transfer_operations();
        let instance_buffer =
            transfer_ctx.create_acceleration_structure_instance_buffer(instances)?;
        transfer_ctx.submit_and_wait()?;

        let instance_buffer_addr = unsafe { self.get_buffer_address(instance_buffer.handle) };

        let geometry_instances = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instance_buffer_addr,
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: geometry_instances.build(),
            });

        let geometries = [&geometry.build()];

        let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .flags(flags)
            .geometries_ptrs(&geometries)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .src_acceleration_structure(vk::AccelerationStructureKHR::null());

        let size_info = unsafe {
            self.ext_acceleration_structure
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_info,
                    &[instances.len() as _],
                )
        };

        let scratch_buffer =
            self.alloc_acceleration_structure_scratch_buffer(size_info.build_scratch_size)?;
        let scratch_address = unsafe { self.get_buffer_address(scratch_buffer.handle) };

        let tlas_buffer =
            self.alloc_acceleration_structure_buffer(size_info.acceleration_structure_size)?;
        let tlas_handle = unsafe {
            self.ext_acceleration_structure
                .create_acceleration_structure(
                    &vk::AccelerationStructureCreateInfoKHR::builder()
                        .buffer(tlas_buffer.handle)
                        .offset(0)
                        .size(size_info.acceleration_structure_size)
                        .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
                        .device_address(vk::DeviceAddress::default()),
                    None,
                )?
        };

        build_info = build_info
            .src_acceleration_structure(vk::AccelerationStructureKHR::null())
            .dst_acceleration_structure(tlas_handle)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(instances.len() as _)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0)
            .build();

        let build_ranges = from_ref(&build_range_info);
        let build_ranges = [build_ranges];

        let compute = self.queues.compute();
        let cmds = unsafe {
            self.record_one_time_command_buffer(compute.pool, |cmds| {
                self.ext_acceleration_structure
                    .cmd_build_acceleration_structures(cmds, from_ref(&build_info), &build_ranges);
                Ok(())
            })?
        };

        unsafe {
            self.submit_and_wait(compute.queue, cmds, &[], &[], &[])?;
            self.free_buffer(scratch_buffer);
            self.free_buffer(instance_buffer);
        }

        Ok(Tlas(AsData {
            buffer: tlas_buffer,
            handle: tlas_handle,
        }))
    }
}
