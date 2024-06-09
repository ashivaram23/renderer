use rand::{thread_rng, Rng};

use crate::{
    objects::{Hit, Ray},
    scene::Scene,
};
use glam::Vec3;

/// Estimates the incident radiance at a pixel (ray.origin) from the camera ray (ray.direction)
pub fn radiance(ray: Ray, scene: &Scene, max_depth: u32) -> Vec3 {
    // Traces the first light bounce based on the camera ray
    let Some(mut hit) = scene.trace_ray(&ray) else {
        return scene.environment;
    };

    // Initializes radiance and throughput multiplier based on first hit
    let mut prev_direction = -ray.direction;
    let mut radiance = scene.object_id(hit.id).material().emitted(&prev_direction);
    let mut throughput = Vec3::ONE;

    // Estimates the radiance contribution from all light paths by sampling two of each length
    for path_length in 2..=max_depth {
        // Estimates path contribution by sampling the BSDF at the hit point
        let (bsdf_sample_multiplier, bsdf_sample_emission, bsdf_sample_weight, next_ray, next_hit) =
            importance_sample_bsdf(&hit, &prev_direction, scene);

        // Estimates path contribution by sampling among the lights in the scene
        let (light_sample_multiplier, light_sample_emission, light_sample_weight) =
            importance_sample_lights(&hit, &prev_direction, scene);

        // Multiplies the weighted estimates by the throughput and adds to the radiance
        radiance += (bsdf_sample_weight * bsdf_sample_multiplier * bsdf_sample_emission
            + light_sample_weight * light_sample_multiplier * light_sample_emission)
            * throughput;

        // Stops if path can't continue because it hit the environment or a non-reflecting emitter
        let Some(next_hit) = next_hit else {
            break;
        };

        // Updates the stored hits (using the new bounce from the BSDF sample) and the throughput
        hit = next_hit;
        prev_direction = -next_ray.direction;
        throughput *= bsdf_sample_multiplier;

        // Decides whether to continue or break early with Russian roulette
        let continue_prob = throughput.max_element().min(0.9);
        if path_length >= 4 && thread_rng().gen::<f32>() < continue_prob {
            throughput /= continue_prob;
        } else if path_length >= 4 {
            break;
        }
    }

    radiance
}

/// Estimates the light scattered at a point by sampling its BSDF for a new light bounce direction
fn importance_sample_bsdf(
    hit: &Hit,
    prev_direction: &Vec3,
    scene: &Scene,
) -> (Vec3, Vec3, f32, Ray, Option<Hit>) {
    // Samples the BSDF for a new direction
    let hit_material = scene.object_id(hit.id).material();
    let (next_ray_direction, bsdf_pdf) = hit_material.sample_direction(&hit.normal);
    let next_ray = Ray::new(hit.point, next_ray_direction);

    // Evaluates the BSDF at the original hit point
    let bsdf_multiplier_at_hit =
        hit_material.bsdf_multiplier(&next_ray.direction, prev_direction, &hit.normal) / bsdf_pdf;

    // Traces the new direction to find the next hit point, and returns early if hits environment
    let Some(next_hit) = scene.trace_ray(&next_ray) else {
        return (
            bsdf_multiplier_at_hit,
            scene.environment,
            1.0,
            next_ray,
            None,
        );
    };

    // Gets the light emitted from the next hit
    let emission_from_next_hit = scene
        .object_id(next_hit.id)
        .material()
        .emitted(&-next_ray.direction);

    // Returns early if the next hit is not an emitter, allowing the path to continue
    if emission_from_next_hit.eq(&Vec3::ZERO) {
        return (
            bsdf_multiplier_at_hit,
            emission_from_next_hit,
            0.0,
            next_ray,
            Some(next_hit),
        );
    }

    // Calculates the multiple importance sampling weight for this estimate
    let light_pdf =
        scene
            .object_id(next_hit.id)
            .surface_pdf(hit.point, next_hit.point, next_hit.triangle_id)
            / scene.lights.len() as f32;
    let weight = bsdf_pdf.powi(2) / (bsdf_pdf.powi(2) + light_pdf.powi(2));

    // Returns this estimate's contribution and weight, with None to show the path can't continue
    (
        bsdf_multiplier_at_hit,
        emission_from_next_hit,
        weight,
        next_ray,
        None,
    )
}

/// Estimates the direct illumination scattered at a point by sampling all the scene's lights
fn importance_sample_lights(hit: &Hit, prev_direction: &Vec3, scene: &Scene) -> (Vec3, Vec3, f32) {
    // Samples the scene's lights for a light point
    let Some((light_ray, light_pdf, emission_from_light)) = scene.sample_lights(hit.point) else {
        return (Vec3::ZERO, Vec3::ZERO, 0.0);
    };

    // Evaluates the BSDF at the original hit point
    let hit_material = scene.object_id(hit.id).material();
    let bsdf_multiplier_at_hit =
        hit_material.bsdf_multiplier(&light_ray.direction, prev_direction, &hit.normal) / light_pdf;

    // Calculates the multiple importance sampling weight for this estimate
    let bsdf_pdf: f32 = hit_material.direction_pdf(&light_ray.direction, &hit.normal);
    let weight = light_pdf.powi(2) / (bsdf_pdf.powi(2) + light_pdf.powi(2));

    // Returns this estimate's contribution and weight
    (bsdf_multiplier_at_hit, emission_from_light, weight)
}
