use std::sync::Arc;

use super::hittable::{Hittable, HittableRef};
use super::onb::Onb;
use super::rtweekend::{random_double, PI};
use super::vec3::{dot, random_cosine_direction, random_unit_vector, unit_vector, Point3, Vec3};

pub trait Pdf: Send + Sync {
    fn value(&self, direction: Vec3) -> f64;
    fn generate(&self) -> Vec3;
}

pub type PdfRef = Arc<PdfObject>;

pub fn make_pdf<T: Into<PdfObject>>(pdf: T) -> PdfRef {
    Arc::new(pdf.into())
}

pub struct SpherePdf;

impl Pdf for SpherePdf {
    fn value(&self, _direction: Vec3) -> f64 {
        1.0 / (4.0 * PI)
    }

    fn generate(&self) -> Vec3 {
        random_unit_vector()
    }
}

pub struct CosinePdf {
    uvw: Onb,
}

impl CosinePdf {
    pub fn new(w: Vec3) -> Self {
        Self { uvw: Onb::new(w) }
    }
}

impl Pdf for CosinePdf {
    fn value(&self, direction: Vec3) -> f64 {
        let cosine_theta = dot(unit_vector(direction), self.uvw.w());
        if cosine_theta <= 0.0 {
            0.0
        } else {
            cosine_theta / PI
        }
    }

    fn generate(&self) -> Vec3 {
        self.uvw.transform(random_cosine_direction())
    }
}

pub struct HittablePdf {
    objects: HittableRef,
    origin: Point3,
}

impl HittablePdf {
    pub fn new(objects: HittableRef, origin: Point3) -> Self {
        Self { objects, origin }
    }
}

impl Pdf for HittablePdf {
    fn value(&self, direction: Vec3) -> f64 {
        self.objects.pdf_value(self.origin, direction)
    }

    fn generate(&self) -> Vec3 {
        self.objects.random(self.origin)
    }
}

pub struct MixturePdf {
    p0: PdfRef,
    p1: PdfRef,
}

impl MixturePdf {
    pub fn new(p0: PdfRef, p1: PdfRef) -> Self {
        Self { p0, p1 }
    }
}

impl Pdf for MixturePdf {
    fn value(&self, direction: Vec3) -> f64 {
        0.5 * self.p0.value(direction) + 0.5 * self.p1.value(direction)
    }

    fn generate(&self) -> Vec3 {
        if random_double() < 0.5 {
            self.p0.generate()
        } else {
            self.p1.generate()
        }
    }
}

pub enum PdfObject {
    SpherePdf(SpherePdf),
    CosinePdf(CosinePdf),
    HittablePdf(HittablePdf),
    MixturePdf(MixturePdf),
}

impl From<SpherePdf> for PdfObject {
    fn from(value: SpherePdf) -> Self {
        Self::SpherePdf(value)
    }
}

impl From<CosinePdf> for PdfObject {
    fn from(value: CosinePdf) -> Self {
        Self::CosinePdf(value)
    }
}

impl From<HittablePdf> for PdfObject {
    fn from(value: HittablePdf) -> Self {
        Self::HittablePdf(value)
    }
}

impl From<MixturePdf> for PdfObject {
    fn from(value: MixturePdf) -> Self {
        Self::MixturePdf(value)
    }
}

impl PdfObject {
    pub fn value(&self, direction: Vec3) -> f64 {
        match self {
            PdfObject::SpherePdf(pdf) => pdf.value(direction),
            PdfObject::CosinePdf(pdf) => pdf.value(direction),
            PdfObject::HittablePdf(pdf) => pdf.value(direction),
            PdfObject::MixturePdf(pdf) => pdf.value(direction),
        }
    }

    pub fn generate(&self) -> Vec3 {
        match self {
            PdfObject::SpherePdf(pdf) => pdf.generate(),
            PdfObject::CosinePdf(pdf) => pdf.generate(),
            PdfObject::HittablePdf(pdf) => pdf.generate(),
            PdfObject::MixturePdf(pdf) => pdf.generate(),
        }
    }
}
