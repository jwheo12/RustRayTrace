use std::sync::Arc;

use super::hittable::HitRecord;
use super::pdf::{make_pdf, CosinePdf, PdfRef, SpherePdf};
use super::ray::Ray;
use super::rtweekend::random_double;
use super::texture::{make_tex, SolidColor, TextureRef};
use super::vec3::{
    dot, random_unit_vector, reflect, refract, unit_vector, Color, Point3, Vec3,
};

pub struct ScatterRecord {
    pub attenuation: Color,
    pub pdf_ptr: Option<PdfRef>,
    pub skip_pdf: bool,
    pub skip_pdf_ray: Ray,
}

pub trait Material: Send + Sync {
    fn emitted(&self, _r_in: &Ray, _rec: &HitRecord, _u: f64, _v: f64, _p: Point3) -> Color {
        Color::new(0.0, 0.0, 0.0)
    }

    fn scatter(&self, _r_in: &Ray, _rec: &HitRecord) -> Option<ScatterRecord> {
        None
    }

    fn scattering_pdf(&self, _r_in: &Ray, _rec: &HitRecord, _scattered: &Ray) -> f64 {
        0.0
    }
}

pub type MaterialRef = Arc<MaterialObject>;

pub fn make_mat<T: Into<MaterialObject>>(material: T) -> MaterialRef {
    Arc::new(material.into())
}

pub struct Lambertian {
    tex: TextureRef,
}

impl Lambertian {
    pub fn new(albedo: Color) -> Self {
        Self { tex: make_tex(SolidColor::new(albedo)) }
    }

    pub fn from_texture(tex: TextureRef) -> Self {
        Self { tex }
    }
}

impl Material for Lambertian {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterRecord> {
        Some(ScatterRecord {
            attenuation: self.tex.value(rec.u, rec.v, rec.p),
            pdf_ptr: Some(make_pdf(CosinePdf::new(rec.normal))),
            skip_pdf: false,
            skip_pdf_ray: Ray::new_with_time(rec.p, rec.normal, r_in.time()),
        })
    }

    fn scattering_pdf(&self, _r_in: &Ray, rec: &HitRecord, scattered: &Ray) -> f64 {
        let cos_theta = dot(rec.normal, unit_vector(scattered.direction()));
        if cos_theta < 0.0 {
            0.0
        } else {
            cos_theta / super::rtweekend::PI
        }
    }
}

pub struct Metal {
    albedo: Color,
    fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Color, fuzz: f64) -> Self {
        Self { albedo, fuzz: if fuzz < 1.0 { fuzz } else { 1.0 } }
    }
}

impl Material for Metal {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterRecord> {
        let reflected = reflect(r_in.direction(), rec.normal);
        let reflected = unit_vector(reflected) + self.fuzz * random_unit_vector();

        Some(ScatterRecord {
            attenuation: self.albedo,
            pdf_ptr: None,
            skip_pdf: true,
            skip_pdf_ray: Ray::new_with_time(rec.p, reflected, r_in.time()),
        })
    }
}

pub struct Dielectric {
    refraction_index: f64,
}

impl Dielectric {
    pub fn new(refraction_index: f64) -> Self {
        Self { refraction_index }
    }

    fn reflectance(cosine: f64, refraction_index: f64) -> f64 {
        let mut r0 = (1.0 - refraction_index) / (1.0 + refraction_index);
        r0 *= r0;
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}

impl Material for Dielectric {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterRecord> {
        let attenuation = Color::new(1.0, 1.0, 1.0);
        let ri = if rec.front_face { 1.0 / self.refraction_index } else { self.refraction_index };

        let unit_direction = unit_vector(r_in.direction());
        let cos_theta = (-dot(unit_direction, rec.normal)).min(1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = ri * sin_theta > 1.0;
        let direction = if cannot_refract || Self::reflectance(cos_theta, ri) > random_double() {
            reflect(unit_direction, rec.normal)
        } else {
            refract(unit_direction, rec.normal, ri)
        };

        Some(ScatterRecord {
            attenuation,
            pdf_ptr: None,
            skip_pdf: true,
            skip_pdf_ray: Ray::new_with_time(rec.p, direction, r_in.time()),
        })
    }
}

pub struct DiffuseLight {
    tex: TextureRef,
}

impl DiffuseLight {
    pub fn new(emit: Color) -> Self {
        Self { tex: make_tex(SolidColor::new(emit)) }
    }

    #[allow(dead_code)]
    pub fn from_texture(tex: TextureRef) -> Self {
        Self { tex }
    }
}

impl Material for DiffuseLight {
    fn emitted(&self, _r_in: &Ray, rec: &HitRecord, u: f64, v: f64, p: Point3) -> Color {
        if !rec.front_face {
            return Color::new(0.0, 0.0, 0.0);
        }
        self.tex.value(u, v, p)
    }
}

pub struct Isotropic {
    tex: TextureRef,
}

impl Isotropic {
    pub fn new(albedo: Color) -> Self {
        Self { tex: make_tex(SolidColor::new(albedo)) }
    }

    pub fn from_texture(tex: TextureRef) -> Self {
        Self { tex }
    }
}

impl Material for Isotropic {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterRecord> {
        Some(ScatterRecord {
            attenuation: self.tex.value(rec.u, rec.v, rec.p),
            pdf_ptr: Some(make_pdf(SpherePdf)),
            skip_pdf: false,
            skip_pdf_ray: Ray::new_with_time(rec.p, Vec3::new(1.0, 0.0, 0.0), r_in.time()),
        })
    }

    fn scattering_pdf(&self, _r_in: &Ray, _rec: &HitRecord, _scattered: &Ray) -> f64 {
        1.0 / (4.0 * super::rtweekend::PI)
    }
}

pub struct EmptyMaterial;
impl Material for EmptyMaterial {}

pub enum MaterialObject {
    Lambertian(Lambertian),
    Metal(Metal),
    Dielectric(Dielectric),
    DiffuseLight(DiffuseLight),
    Isotropic(Isotropic),
    EmptyMaterial(EmptyMaterial),
}

impl From<Lambertian> for MaterialObject {
    fn from(value: Lambertian) -> Self {
        Self::Lambertian(value)
    }
}

impl From<Metal> for MaterialObject {
    fn from(value: Metal) -> Self {
        Self::Metal(value)
    }
}

impl From<Dielectric> for MaterialObject {
    fn from(value: Dielectric) -> Self {
        Self::Dielectric(value)
    }
}

impl From<DiffuseLight> for MaterialObject {
    fn from(value: DiffuseLight) -> Self {
        Self::DiffuseLight(value)
    }
}

impl From<Isotropic> for MaterialObject {
    fn from(value: Isotropic) -> Self {
        Self::Isotropic(value)
    }
}

impl From<EmptyMaterial> for MaterialObject {
    fn from(value: EmptyMaterial) -> Self {
        Self::EmptyMaterial(value)
    }
}

impl MaterialObject {
    pub fn emitted(&self, r_in: &Ray, rec: &HitRecord, u: f64, v: f64, p: Point3) -> Color {
        match self {
            MaterialObject::Lambertian(mat) => mat.emitted(r_in, rec, u, v, p),
            MaterialObject::Metal(mat) => mat.emitted(r_in, rec, u, v, p),
            MaterialObject::Dielectric(mat) => mat.emitted(r_in, rec, u, v, p),
            MaterialObject::DiffuseLight(mat) => mat.emitted(r_in, rec, u, v, p),
            MaterialObject::Isotropic(mat) => mat.emitted(r_in, rec, u, v, p),
            MaterialObject::EmptyMaterial(mat) => mat.emitted(r_in, rec, u, v, p),
        }
    }

    pub fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterRecord> {
        match self {
            MaterialObject::Lambertian(mat) => mat.scatter(r_in, rec),
            MaterialObject::Metal(mat) => mat.scatter(r_in, rec),
            MaterialObject::Dielectric(mat) => mat.scatter(r_in, rec),
            MaterialObject::DiffuseLight(mat) => mat.scatter(r_in, rec),
            MaterialObject::Isotropic(mat) => mat.scatter(r_in, rec),
            MaterialObject::EmptyMaterial(mat) => mat.scatter(r_in, rec),
        }
    }

    pub fn scattering_pdf(&self, r_in: &Ray, rec: &HitRecord, scattered: &Ray) -> f64 {
        match self {
            MaterialObject::Lambertian(mat) => mat.scattering_pdf(r_in, rec, scattered),
            MaterialObject::Metal(mat) => mat.scattering_pdf(r_in, rec, scattered),
            MaterialObject::Dielectric(mat) => mat.scattering_pdf(r_in, rec, scattered),
            MaterialObject::DiffuseLight(mat) => mat.scattering_pdf(r_in, rec, scattered),
            MaterialObject::Isotropic(mat) => mat.scattering_pdf(r_in, rec, scattered),
            MaterialObject::EmptyMaterial(mat) => mat.scattering_pdf(r_in, rec, scattered),
        }
    }
}
