use std::slice::from_ref;
use std::sync::{Arc, Weak};

use bevy_asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy_reflect::List;
use gltf::Semantic;

use pompeii::{errors::PompeiiError, mesh::VertexPosNormUvF32, PompeiiRenderer};

use crate::MeshAsset;

pub struct GltfLoader {
    renderer: Weak<PompeiiRenderer>,
}

impl From<Weak<PompeiiRenderer>> for GltfLoader {
    fn from(renderer: Weak<PompeiiRenderer>) -> Self {
        Self { renderer }
    }
}

impl AssetLoader for GltfLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let (doc, buffers, _) = gltf::import_slice(bytes)?;

            // TODO: complete loader

            let gltf_mesh = doc.meshes().next().unwrap();
            let mut sub_meshes = Vec::with_capacity(gltf_mesh.primitives().len());

            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for sub_mesh in gltf_mesh.primitives() {
                let vert_start = vertices.len();
                let index_start = indices.len();

                let reader = sub_mesh.reader(|buf| Some(&buffers[buf.index()]));

                // Sanity check
                let pos_count = sub_mesh
                    .get(&Semantic::Positions)
                    .ok_or(PompeiiError::NoVertexPosition)?
                    .count();
                let norm_count = sub_mesh
                    .get(&Semantic::Normals)
                    .ok_or(PompeiiError::NoVertexNormal)?
                    .count();
                let uv_count = sub_mesh
                    .get(&Semantic::TexCoords(0))
                    .ok_or(PompeiiError::NoVertexUv)?
                    .count();
                assert_eq!(pos_count, norm_count);
                assert_eq!(norm_count, uv_count);

                // Prepare components reader
                let pos = reader.read_positions().unwrap();
                let norm = reader.read_normals().unwrap();
                let uv = reader.read_tex_coords(0).unwrap();
                let index = reader.read_indices().unwrap();

                // Transform into vertices
                for (((pos, norm), uv), index) in
                    pos.zip(norm).zip(uv.into_f32()).zip(index.into_u32())
                {
                    vertices.push(VertexPosNormUvF32 { pos, norm, uv });
                    indices.push(index as u16);
                }

                let vert_count = vertices.len() - vert_start;
                let index_count = indices.len() - index_start;

                sub_meshes.push((vert_start, vert_count, index_start, index_count));
            }

            let renderer = self.renderer.upgrade().unwrap();

            let mut transfer_ctx = renderer.start_transfer_operations();
            let vertices_handle = transfer_ctx.create_vertex_buffer(&vertices)?;
            let indices_handle = transfer_ctx.create_index_buffer(&indices)?;
            transfer_ctx.submit_and_wait()?;

            let mesh =
                renderer.create_mesh(vertices_handle, indices_handle, sub_meshes.into_iter());

            // TODO
            // let blas = renderer.create_blas(from_ref(&mesh));

            load_context.set_default_asset(LoadedAsset::new(MeshAsset {
                renderer: Arc::downgrade(&renderer),
                mesh,
            }));

            drop(renderer);

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["glb", "gltf"]
    }
}
