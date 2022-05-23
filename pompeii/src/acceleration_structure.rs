use std::slice::from_ref;

use ash::vk;

use crate::errors::Result;
use crate::mesh::{Mesh, MeshIndex, MeshVertex, SubMesh, VertexPosNormUvF32};
use crate::PompeiiRenderer;

pub struct Blas {}

#[derive(Debug, Default)]
pub struct BlasInput {
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    build_ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
}

struct AsBuildInfo<'a> {
    build_info: vk::AccelerationStructureBuildGeometryInfoKHR,
    range_info: &'a [vk::AccelerationStructureBuildRangeInfoKHR],
    size_info: vk::AccelerationStructureBuildSizesInfoKHR,
}

impl PompeiiRenderer {
    pub fn create_blas<'a>(&self, meshes: impl Iterator<Item = &'a Mesh>) {
        let blas_inputs = meshes
            .map(|mesh| self.object_to_vk_geometry(mesh))
            .collect::<Vec<_>>();
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
    ) -> Result<()> {
        let build_infos = inputs
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
                    size_info: size_info,
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

        let transfer = self.queues.transfer();
        let cmds = unsafe {
            self.record_one_time_command_buffer(transfer.pool, |cmds| {
                self.cmd_build_blas(cmds, build_infos, scratch_address);
                Ok(())
            })
        }?;

        unsafe {
            self.submit_and_wait(
                transfer.queue,
                cmds,
                &[],
                &[],
                &[],
            )?;
        }

        Ok(())
    }

    unsafe fn cmd_build_blas<'a>(
        &self,
        cmds: vk::CommandBuffer,
        build_infos: impl IntoIterator<Item = AsBuildInfo<'a>>,
        scratch_address: vk::DeviceAddress,
    ) -> Result<()> {
        for mut build_info in build_infos {
            let size = build_info.size_info.acceleration_structure_size;

            let as_buffer = self.alloc_acceleration_structure(size)?;
            let as_handle = self
                .ext_acceleration_structure
                .create_acceleration_structure(
                    &vk::AccelerationStructureCreateInfoKHR::builder()
                        .buffer(as_buffer.handle)
                        .offset(0)
                        .size(size)
                        .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                        .device_address(vk::DeviceAddress::default()),
                    None,
                )?;

            build_info.build_info.dst_acceleration_structure = as_handle;
            build_info.build_info.scratch_data = vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            };

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

// Extend mesh with cool methods
impl SubMesh {
    pub(crate) fn max_vertex_index(&self) -> u32 {
        (self.index_start + self.index_count - 1) as _
    }
}