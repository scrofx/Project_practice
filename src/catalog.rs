use diesel::prelude::*;
use rocket::{get, http::Status, serde::json::Json};

use crate::{
    db::Db,
    models::{CampusBuilding, Category},
    schema::{campus_buildings, categories},
};
use campuslost_shared::{CategoryDto, CampusBuildingDto};


fn category_to_dto(category: Category) -> CategoryDto {
    CategoryDto {
        id: category.id,
        name: category.name,
        slug: category.slug,
        icon: category.icon,
        is_active: category.is_active,
        created_at: category.created_at,
    }
}

fn building_to_dto(building: CampusBuilding) -> CampusBuildingDto {
    CampusBuildingDto {
        id: building.id,
        name: building.name,
        code: building.code,
        address: building.address,
        is_active: building.is_active,
        created_at: building.created_at,
    }
}


// route for frontend to get existing categories of lost items. Allows passing of uuid of a category
// in a request, rather than a name (which will force a scan of categories table to find an id)
#[get("/categories")]
pub async fn get_categories(db: Db) -> Result<Json<Vec<CategoryDto>>, Status> {
    let result = db
        .run(|conn| {
            categories::table
                .filter(categories::is_active.eq(true))
                .order(categories::name.asc())
                .select(Category::as_select())
                .load::<Category>(conn)
        })
        .await;

    match result {
        Ok(items) => {
            let dtos = items
                .into_iter()
                .map(category_to_dto)
                .collect();

            Ok(Json(dtos))
        }
        Err(_) => Err(Status::InternalServerError),
    }
}

// route for frontend to get existing buildings. Allows passing of uuid of a building
// in a request, rather than a name (which will force a scan of buildings table to find an id)
#[get("/buildings")]
pub async fn get_buildings(db: Db) -> Result<Json<Vec<CampusBuildingDto>>, Status> {
    let result = db
        .run(|conn| {
            campus_buildings::table
                .filter(campus_buildings::is_active.eq(true))
                .order(campus_buildings::name.asc())
                .select(CampusBuilding::as_select())
                .load::<CampusBuilding>(conn)
        })
        .await;

    match result {
        Ok(items) => {
            let dtos = items
                .into_iter()
                .map(building_to_dto)
                .collect();

            Ok(Json(dtos))
        }
        Err(_) => Err(Status::InternalServerError),
    }
}
