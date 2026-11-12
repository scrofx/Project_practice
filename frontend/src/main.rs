use campuslost_shared::{ApiError, AuthResponse, CampusBuildingDto, CategoryDto, CreateListingRequest, ListingDto, ListingStatus, ListingType, LoginRequest, MatchAction, MatchActionRequest, MatchActionResponse, MatchContactsResponse, MatchDto, MatchStatus, RegisterRequest};
use chrono::NaiveDate;
use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

const API_BASE: &str = "http://127.0.0.1:8000";

#[function_component(ListingsPage)]
fn listings_page() -> Html {
    let listings = use_state(|| None::<Result<Vec<ListingDto>, String>>);

    // Feed search and filters
    let search_query = use_state(String::new);
    let type_filter = use_state(|| "All".to_string());
    let category_filter = use_state(|| "All".to_string());
    let building_filter = use_state(|| "All".to_string());
    let date_from = use_state(String::new);
    let date_to = use_state(String::new);
    let include_archived = use_state(|| false);

    // Catalog data
    let categories = use_state(|| None::<Vec<CategoryDto>>);
    let buildings = use_state(|| None::<Vec<CampusBuildingDto>>);
    let catalog_error = use_state(|| None::<String>);

    // Create-listing form
    let show_create_form = use_state(|| false);
    let submit_message = use_state(|| None::<(bool, String)>);
    let title_ref = use_node_ref();
    let description_ref = use_node_ref();
    let location_detail_ref = use_node_ref();
    let event_date_ref = use_node_ref();
    let selected_type = use_state(|| "Lost".to_string());
    let selected_category = use_state(String::new);
    let selected_building = use_state(String::new);

    // Load public listings.
    {
        let listings = listings.clone();

        use_effect_with(*include_archived, move |include_archived| {
            let include_archived = *include_archived;

            spawn_local(async move {
                let result = async {
                    let url = if include_archived {
                        format!("{API_BASE}/listings?include_archived=true")
                    } else {
                        format!("{API_BASE}/listings")
                    };

                    let response = Request::get(&url)
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;

                    if !response.ok() {
                        return Err(format!("Server returned status {}", response.status()));
                    }

                    response
                        .json::<Vec<ListingDto>>()
                        .await
                        .map_err(|error| error.to_string())
                }
                    .await;

                listings.set(Some(result));
            });

            || ()
        });
    }

    // Load categories and buildings used by the create form and feed filters.
    {
        let categories = categories.clone();
        let buildings = buildings.clone();
        let catalog_error = catalog_error.clone();
        let selected_category = selected_category.clone();
        let selected_building = selected_building.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let result = async {
                    let categories_response =
                        Request::get(&format!("{API_BASE}/categories"))
                            .send()
                            .await
                            .map_err(|error| error.to_string())?;

                    if !categories_response.ok() {
                        return Err(format!(
                            "Categories request failed: {}",
                            categories_response.status()
                        ));
                    }

                    let category_items = categories_response
                        .json::<Vec<CategoryDto>>()
                        .await
                        .map_err(|error| error.to_string())?;

                    let buildings_response =
                        Request::get(&format!("{API_BASE}/buildings"))
                            .send()
                            .await
                            .map_err(|error| error.to_string())?;

                    if !buildings_response.ok() {
                        return Err(format!(
                            "Buildings request failed: {}",
                            buildings_response.status()
                        ));
                    }

                    let building_items = buildings_response
                        .json::<Vec<CampusBuildingDto>>()
                        .await
                        .map_err(|error| error.to_string())?;

                    Ok((category_items, building_items))
                }
                    .await;

                match result {
                    Ok((category_items, building_items)) => {
                        if let Some(category) =
                            category_items.iter().find(|item| item.is_active)
                        {
                            selected_category.set(category.id.to_string());
                        }

                        if let Some(building) =
                            building_items.iter().find(|item| item.is_active)
                        {
                            selected_building.set(building.id.to_string());
                        }

                        categories.set(Some(category_items));
                        buildings.set(Some(building_items));
                    }
                    Err(error) => catalog_error.set(Some(error)),
                }
            });

            || ()
        });
    }

    html! {
        <div class="container py-4">
            <div class="d-flex justify-content-between align-items-center mb-4 flex-wrap gap-2">
                <h2 class="mb-0">{"Lost and Found Listings"}</h2>

                <button
                    type="button"
                    class="btn btn-primary"
                    onclick={{
                        let show_create_form = show_create_form.clone();
                        Callback::from(move |_| {
                            show_create_form.set(!*show_create_form);
                        })
                    }}
                >
                    {
                        if *show_create_form {
                            "Cancel"
                        } else {
                            "+ Create Listing"
                        }
                    }
                </button>
            </div>

            // Search field
            <div class="row mb-4">
                <div class="col-12">
                    <label for="listing-search" class="form-label">
                        {"Search listings"}
                    </label>
                    <input
                        id="listing-search"
                        type="search"
                        class="form-control"
                        placeholder="Search by title or description..."
                        value={(*search_query).clone()}
                        oninput={{
                            let search_query = search_query.clone();
                            Callback::from(move |event: InputEvent| {
                                let input: HtmlInputElement = event.target_unchecked_into();
                                search_query.set(input.value());
                            })
                        }}
                    />
                </div>
            </div>

            {
                if let Some(error) = &*catalog_error {
                    html! {
                        <div class="alert alert-danger">
                            {format!("Could not load categories and buildings: {error}")}
                        </div>
                    }
                } else {
                    Html::default()
                }
            }

            {
                if let Some((is_error, message)) = &*submit_message {
                    let alert_class = if *is_error {
                        "alert alert-danger"
                    } else {
                        "alert alert-success"
                    };

                    html! {
                        <div class={alert_class}>
                            {message.clone()}
                        </div>
                    }
                } else {
                    Html::default()
                }
            }

            // Create-listing form
            {
                if *show_create_form {
                    html! {
                        <div class="card shadow-sm mb-4">
                            <div class="card-header bg-primary text-white">
                                <h5 class="mb-0">{"Create a Listing"}</h5>
                            </div>

                            <div class="card-body">
                                <div class="mb-3">
                                    <label class="form-label">{"Listing Type"}</label>
                                    <select
                                        class="form-select"
                                        value={(*selected_type).clone()}
                                        onchange={{
                                            let selected_type = selected_type.clone();
                                            Callback::from(move |event: Event| {
                                                let select: HtmlSelectElement =
                                                    event.target_unchecked_into();
                                                selected_type.set(select.value());
                                            })
                                        }}
                                    >
                                        <option value="Lost">{"Lost item"}</option>
                                        <option value="Found">{"Found item"}</option>
                                    </select>
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">{"Title"}</label>
                                    <input
                                        type="text"
                                        class="form-control"
                                        placeholder="e.g. Black backpack"
                                        ref={title_ref.clone()}
                                    />
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">{"Description"}</label>
                                    <textarea
                                        class="form-control"
                                        rows="3"
                                        placeholder="Describe the item..."
                                        ref={description_ref.clone()}
                                    />
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">{"Category"}</label>
                                    <select
                                        class="form-select"
                                        value={(*selected_category).clone()}
                                        onchange={{
                                            let selected_category = selected_category.clone();
                                            Callback::from(move |event: Event| {
                                                let select: HtmlSelectElement =
                                                    event.target_unchecked_into();
                                                selected_category.set(select.value());
                                            })
                                        }}
                                    >
                                        <option value="">{"Select a category"}</option>
                                        {
                                            if let Some(items) = &*categories {
                                                items
                                                    .iter()
                                                    .filter(|item| item.is_active)
                                                    .map(|item| html! {
                                                        <option value={item.id.to_string()}>
                                                            {item.name.clone()}
                                                        </option>
                                                    })
                                                    .collect::<Html>()
                                            } else {
                                                Html::default()
                                            }
                                        }
                                    </select>
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">{"Building"}</label>
                                    <select
                                        class="form-select"
                                        value={(*selected_building).clone()}
                                        onchange={{
                                            let selected_building = selected_building.clone();
                                            Callback::from(move |event: Event| {
                                                let select: HtmlSelectElement =
                                                    event.target_unchecked_into();
                                                selected_building.set(select.value());
                                            })
                                        }}
                                    >
                                        <option value="">{"Select a building"}</option>
                                        {
                                            if let Some(items) = &*buildings {
                                                items
                                                    .iter()
                                                    .filter(|item| item.is_active)
                                                    .map(|item| html! {
                                                        <option value={item.id.to_string()}>
                                                            {item.name.clone()}
                                                        </option>
                                                    })
                                                    .collect::<Html>()
                                            } else {
                                                Html::default()
                                            }
                                        }
                                    </select>
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">{"Date"}</label>
                                    <input
                                        type="date"
                                        class="form-control"
                                        ref={event_date_ref.clone()}
                                    />
                                </div>

                                <div class="mb-3">
                                    <label class="form-label">
                                        {"Additional Location Details (optional)"}
                                    </label>
                                    <input
                                        type="text"
                                        class="form-control"
                                        placeholder="e.g. Room 204, near the entrance"
                                        ref={location_detail_ref.clone()}
                                    />
                                </div>

                                <div class="d-grid mt-4">
                                    <button
                                        type="button"
                                        class="btn btn-success"
                                        onclick={{
                                            let title_ref = title_ref.clone();
                                            let description_ref = description_ref.clone();
                                            let location_detail_ref = location_detail_ref.clone();
                                            let event_date_ref = event_date_ref.clone();
                                            let selected_type = selected_type.clone();
                                            let selected_category = selected_category.clone();
                                            let selected_building = selected_building.clone();
                                            let submit_message = submit_message.clone();

                                            Callback::from(move |_| {
                                                let title = title_ref
                                                    .cast::<HtmlInputElement>()
                                                    .map(|input| input.value())
                                                    .unwrap_or_default();

                                                let description = description_ref
                                                    .cast::<HtmlTextAreaElement>()
                                                    .map(|input| input.value())
                                                    .unwrap_or_default();

                                                let location_detail = location_detail_ref
                                                    .cast::<HtmlInputElement>()
                                                    .map(|input| input.value())
                                                    .unwrap_or_default();

                                                let event_date_value = event_date_ref
                                                    .cast::<HtmlInputElement>()
                                                    .map(|input| input.value())
                                                    .unwrap_or_default();

                                                if title.trim().is_empty()
                                                    || description.trim().is_empty()
                                                    || selected_category.is_empty()
                                                    || selected_building.is_empty()
                                                    || event_date_value.is_empty()
                                                {
                                                    submit_message.set(Some((
                                                        true,
                                                        "Please fill in all required fields.".to_string(),
                                                    )));
                                                    return;
                                                }

                                                let category_id =
                                                    match Uuid::parse_str(&selected_category) {
                                                        Ok(id) => id,
                                                        Err(_) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                "Please select a valid category."
                                                                    .to_string(),
                                                            )));
                                                            return;
                                                        }
                                                    };

                                                let building_id =
                                                    match Uuid::parse_str(&selected_building) {
                                                        Ok(id) => id,
                                                        Err(_) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                "Please select a valid building."
                                                                    .to_string(),
                                                            )));
                                                            return;
                                                        }
                                                    };

                                                let event_date =
                                                    match NaiveDate::parse_from_str(
                                                        &event_date_value,
                                                        "%Y-%m-%d",
                                                    ) {
                                                        Ok(date) => date,
                                                        Err(_) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                "Please enter a valid date."
                                                                    .to_string(),
                                                            )));
                                                            return;
                                                        }
                                                    };

                                                let listing_type = if *selected_type == "Found" {
                                                    ListingType::Found
                                                } else {
                                                    ListingType::Lost
                                                };

                                                let payload = CreateListingRequest {
                                                    listing_type,
                                                    category_id,
                                                    building_id,
                                                    title: title.trim().to_string(),
                                                    description: description.trim().to_string(),
                                                    location_detail: if location_detail
                                                        .trim()
                                                        .is_empty()
                                                    {
                                                        None
                                                    } else {
                                                        Some(location_detail.trim().to_string())
                                                    },
                                                    event_date,
                                                };

                                                let token: String =
                                                    match LocalStorage::get("campuslost_token") {
                                                        Ok(token) => token,
                                                        Err(_) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                "You are not logged in. Please log in again."
                                                                    .to_string(),
                                                            )));
                                                            return;
                                                        }
                                                    };

                                                let submit_message = submit_message.clone();
                                                let title_ref = title_ref.clone();
                                                let description_ref = description_ref.clone();
                                                let location_detail_ref =
                                                    location_detail_ref.clone();
                                                let event_date_ref = event_date_ref.clone();

                                                spawn_local(async move {
                                                    let request = Request::post(&format!(
                                                        "{API_BASE}/listings"
                                                    ))
                                                    .header(
                                                        "Authorization",
                                                        &format!("Bearer {token}"),
                                                    )
                                                    .json(&payload);

                                                    let response = match request {
                                                        Ok(request) => request.send().await,
                                                        Err(error) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                format!(
                                                                    "Could not prepare request: {error}"
                                                                ),
                                                            )));
                                                            return;
                                                        }
                                                    };

                                                    match response {
                                                        Ok(response) if response.ok() => {
                                                            if let Some(input) =
                                                                title_ref.cast::<HtmlInputElement>()
                                                            {
                                                                input.set_value("");
                                                            }

                                                            if let Some(input) = description_ref
                                                                .cast::<HtmlTextAreaElement>()
                                                            {
                                                                input.set_value("");
                                                            }

                                                            if let Some(input) = location_detail_ref
                                                                .cast::<HtmlInputElement>()
                                                            {
                                                                input.set_value("");
                                                            }

                                                            if let Some(input) = event_date_ref
                                                                .cast::<HtmlInputElement>()
                                                            {
                                                                input.set_value("");
                                                            }

                                                            submit_message.set(Some((
                                                                false,
                                                                "Listing submitted successfully! It is pending moderator review."
                                                                    .to_string(),
                                                            )));
                                                        }
                                                        Ok(response) => {
                                                            let status = response.status();
                                                            let body = response
                                                                .text()
                                                                .await
                                                                .unwrap_or_default();

                                                            submit_message.set(Some((
                                                                true,
                                                                format!(
                                                                    "Server returned {status}: {body}"
                                                                ),
                                                            )));
                                                        }
                                                        Err(error) => {
                                                            submit_message.set(Some((
                                                                true,
                                                                format!(
                                                                    "Could not connect to the server: {error}"
                                                                ),
                                                            )));
                                                        }
                                                    }
                                                });
                                            })
                                        }}
                                    >
                                        {"Submit Listing"}
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    Html::default()
                }
            }

            // Listings feed and filters
            {
                match &*listings {
                    None => html! {
                        <div class="alert alert-info">
                            {"Loading listings..."}
                        </div>
                    },

                    Some(Err(error)) => html! {
                        <div class="alert alert-danger">
                            {format!("Could not load listings: {error}")}
                        </div>
                    },

                    Some(Ok(items)) if items.is_empty() => html! {
                        <div class="alert alert-secondary">
                            {"No listings available yet."}
                        </div>
                    },

                    Some(Ok(items)) => {
                        let query = search_query.to_lowercase();
                        let selected_type_filter = (*type_filter).clone();
                        let selected_category_id = (*category_filter).clone();
                        let selected_building_id = (*building_filter).clone();
                        let from_date = (*date_from).clone();
                        let to_date = (*date_to).clone();

                        let filtered_items: Vec<&ListingDto> = items
                            .iter()
                            .filter(|item| {
                                let matches_search =
                                    item.title.to_lowercase().contains(&query)
                                    || item.description.to_lowercase().contains(&query);

                                let matches_type = selected_type_filter == "All"
                                    || match item.listing_type {
                                        ListingType::Lost => selected_type_filter == "Lost",
                                        ListingType::Found => selected_type_filter == "Found",
                                    };

                                let matches_category = selected_category_id == "All"
                                    || item.category_id.to_string() == selected_category_id;

                                let matches_building = selected_building_id == "All"
                                    || item.building_id.to_string() == selected_building_id;

                                // YYYY-MM-DD dates can be compared lexicographically.
                                let item_date = item.event_date.to_string();
                                let matches_from_date =
                                    from_date.is_empty() || item_date >= from_date;
                                let matches_to_date =
                                    to_date.is_empty() || item_date <= to_date;

                                matches_search
                                    && matches_type
                                    && matches_category
                                    && matches_building
                                    && matches_from_date
                                    && matches_to_date
                            })
                            .collect();

                        html! {
                            <>
                                <div class="row mb-3">
                                    <div class="col-12 col-md-4">
                                        <label for="type-filter" class="form-label">
                                            {"Listing type"}
                                        </label>
                                        <select
                                            id="type-filter"
                                            class="form-select"
                                            value={(*type_filter).clone()}
                                            onchange={{
                                                let type_filter = type_filter.clone();
                                                Callback::from(move |event: Event| {
                                                    let select: HtmlSelectElement =
                                                        event.target_unchecked_into();
                                                    type_filter.set(select.value());
                                                })
                                            }}
                                        >
                                            <option value="All">{"All listings"}</option>
                                            <option value="Lost">{"Lost"}</option>
                                            <option value="Found">{"Found"}</option>
                                        </select>
                                    </div>
                                </div>

                                <div class="row mb-3">
                                    <div class="col-12 col-md-6">
                                        <label for="category-filter" class="form-label">
                                            {"Category"}
                                        </label>
                                        <select
                                            id="category-filter"
                                            class="form-select"
                                            value={(*category_filter).clone()}
                                            onchange={{
                                                let category_filter = category_filter.clone();
                                                Callback::from(move |event: Event| {
                                                    let select: HtmlSelectElement =
                                                        event.target_unchecked_into();
                                                    category_filter.set(select.value());
                                                })
                                            }}
                                        >
                                            <option
                                                value="All"
                                                selected={*category_filter == "All"}
                                            >
                                                {"All categories"}
                                            </option>
                                            {
                                                if let Some(items) = &*categories {
                                                    items
                                                        .iter()
                                                        .filter(|item| item.is_active)
                                                        .map(|item| {
                                                            let id = item.id.to_string();
                                                            html! {
                                                                <option
                                                                    value={id.clone()}
                                                                    selected={*category_filter == id}
                                                                >
                                                                    {item.name.clone()}
                                                                </option>
                                                            }
                                                        })
                                                        .collect::<Html>()
                                                } else {
                                                    Html::default()
                                                }
                                            }
                                        </select>
                                    </div>

                                    <div class="col-12 col-md-6">
                                        <label for="building-filter" class="form-label">
                                            {"Building"}
                                        </label>
                                        <select
                                            id="building-filter"
                                            class="form-select"
                                            value={(*building_filter).clone()}
                                            onchange={{
                                                let building_filter = building_filter.clone();
                                                Callback::from(move |event: Event| {
                                                    let select: HtmlSelectElement =
                                                        event.target_unchecked_into();
                                                    building_filter.set(select.value());
                                                })
                                            }}
                                        >
                                            <option
                                                value="All"
                                                selected={*building_filter == "All"}
                                            >
                                                {"All buildings"}
                                            </option>
                                            {
                                                if let Some(items) = &*buildings {
                                                    items
                                                        .iter()
                                                        .filter(|item| item.is_active)
                                                        .map(|item| {
                                                            let id = item.id.to_string();
                                                            html! {
                                                                <option
                                                                    value={id.clone()}
                                                                    selected={*building_filter == id}
                                                                >
                                                                    {item.name.clone()}
                                                                </option>
                                                            }
                                                        })
                                                        .collect::<Html>()
                                                } else {
                                                    Html::default()
                                                }
                                            }
                                        </select>
                                    </div>
                                </div>

                                // Date range
                                <div class="row mb-3">
                                    <div class="col-12 col-md-6">
                                        <label for="date-from" class="form-label">
                                            {"From date"}
                                        </label>
                                        <input
                                            id="date-from"
                                            type="date"
                                            class="form-control"
                                            value={(*date_from).clone()}
                                            oninput={{
                                                let date_from = date_from.clone();
                                                Callback::from(move |event: InputEvent| {
                                                    let input: HtmlInputElement =
                                                        event.target_unchecked_into();
                                                    date_from.set(input.value());
                                                })
                                            }}
                                        />
                                    </div>

                                    <div class="col-12 col-md-6">
                                        <label for="date-to" class="form-label">
                                            {"To date"}
                                        </label>
                                        <input
                                            id="date-to"
                                            type="date"
                                            class="form-control"
                                            value={(*date_to).clone()}
                                            oninput={{
                                                let date_to = date_to.clone();
                                                Callback::from(move |event: InputEvent| {
                                                    let input: HtmlInputElement =
                                                        event.target_unchecked_into();
                                                    date_to.set(input.value());
                                                })
                                            }}
                                        />
                                    </div>
                                </div>
                                <div class="form-check mt-3">
                                    <input
                                        class="form-check-input"
                                        type="checkbox"
                                        id="include-archived"
                                        checked={*include_archived}
                                        onchange={{
                                            let include_archived = include_archived.clone();
                                            Callback::from(move |event: Event| {
                                                let input = event.target_unchecked_into::<HtmlInputElement>();
                                                include_archived.set(input.checked());
                                            })
                                        }}
                                    />
                                    <label class="form-check-label" for="include-archived">
                                        {"Include archived listings"}
                                    </label>
                                </div>
                                <div class="row mb-3">
                                    <div class="col-12">
                                        <button
                                            type="button"
                                            class="btn btn-outline-secondary"
                                            onclick={{
                                                let search_query = search_query.clone();
                                                let type_filter = type_filter.clone();
                                                let category_filter = category_filter.clone();
                                                let building_filter = building_filter.clone();
                                                let date_from = date_from.clone();
                                                let date_to = date_to.clone();
                                                let include_archived = include_archived.clone();

                                                Callback::from(move |_| {
                                                    search_query.set(String::new());
                                                    type_filter.set("All".to_string());
                                                    category_filter.set("All".to_string());
                                                    building_filter.set("All".to_string());
                                                    date_from.set(String::new());
                                                    date_to.set(String::new());
                                                    include_archived.set(false);
                                                })
                                            }}
                                        >
                                            {"Reset filters"}
                                        </button>
                                    </div>
                                </div>

                                {
                                    if filtered_items.is_empty() {
                                        html! {
                                            <div class="alert alert-secondary">
                                                {"No listings match your filters."}
                                            </div>
                                        }
                                    } else {
                                        html! {
                                            <div class="row g-3">
                                                {
                                                    filtered_items.iter().map(|item| html! {
                                                        <div class="col-12 col-md-6 col-lg-4">
                                                            <div class="card h-100 shadow-sm">
                                                                <div class="card-body">
                                                                    <span class={
                                                                        match item.listing_type {
                                                                            ListingType::Lost =>
                                                                                "badge text-bg-danger",
                                                                            ListingType::Found =>
                                                                                "badge text-bg-success",
                                                                        }
                                                                    }>
                                                                        {
                                                                            match item.listing_type {
                                                                                ListingType::Lost => "LOST",
                                                                                ListingType::Found => "FOUND",
                                                                            }
                                                                        }
                                                                    </span>

                                                                    <h5 class="card-title mt-2">
                                                                        {item.title.clone()}
                                                                    </h5>

                                                                    <p class="card-text">
                                                                        {item.description.clone()}
                                                                    </p>

                                                                    <p class="text-muted small mb-0">
                                                                        {format!("Date: {}", item.event_date)}
                                                                    </p>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }).collect::<Html>()
                                                }
                                            </div>
                                        }
                                    }
                                }
                            </>
                        }
                    }
                }
            }
        </div>
    }
}

#[function_component(MyListingsPage)]
fn my_listings_page() -> Html {
    let listings = use_state(|| None::<Result<Vec<ListingDto>, String>>);

    {
        let listings = listings.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let result = async {
                    let token: String = LocalStorage::get("campuslost_token")
                        .map_err(|error| error.to_string())?;

                    let response = Request::get(&format!(
                        "{API_BASE}/users/me/listings"
                    ))
                        .header("Authorization", &format!("Bearer {token}"))
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;

                    if !response.ok() {
                        return Err(format!(
                            "Server returned status {}",
                            response.status()
                        ));
                    }

                    response
                        .json::<Vec<ListingDto>>()
                        .await
                        .map_err(|error| error.to_string())
                }
                    .await;

                listings.set(Some(result));
            });

            || ()
        });
    }

    html! {
        <div class="container py-4">
            <h2 class="mb-4">{"My Listings"}</h2>

            {
                match &*listings {
                    None => html! {
                        <div class="alert alert-info">
                            {"Loading your listings..."}
                        </div>
                    },

                    Some(Err(error)) => html! {
                        <div class="alert alert-danger">
                            {format!("Could not load your listings: {error}")}
                        </div>
                    },

                    Some(Ok(items)) if items.is_empty() => html! {
                        <div class="alert alert-secondary">
                            {"You haven't submitted any listings yet."}
                        </div>
                    },

                    Some(Ok(items)) => html! {
                        <div class="row g-3">
                            {
                                items.iter().map(|item| {
                                    let (status_label, status_class) =
                                        match item.status {
                                            ListingStatus::PendingReview =>
                                                ("Pending review", "badge text-bg-warning"),
                                            ListingStatus::Active =>
                                                ("Active", "badge text-bg-success"),
                                            ListingStatus::Matched =>
                                                ("Matched", "badge text-bg-primary"),
                                            ListingStatus::Returned =>
                                                ("Returned", "badge text-bg-info"),
                                            ListingStatus::Removed =>
                                                ("Removed", "badge text-bg-danger"),
                                            ListingStatus::Archived =>
                                                ("Archived", "badge text-bg-secondary"),
                                        };

                                    html! {
                                        <div class="col-12 col-md-6 col-lg-4">
                                            <div class="card h-100 shadow-sm">
                                                <div class="card-body">
                                                    <div class="d-flex justify-content-between align-items-center gap-2 mb-2">
                                                        <span class={
                                                            match item.listing_type {
                                                                ListingType::Lost =>
                                                                    "badge text-bg-danger",
                                                                ListingType::Found =>
                                                                    "badge text-bg-success",
                                                            }
                                                        }>
                                                            {
                                                                match item.listing_type {
                                                                    ListingType::Lost => "LOST",
                                                                    ListingType::Found => "FOUND",
                                                                }
                                                            }
                                                        </span>

                                                        <span class={status_class}>
                                                            {status_label}
                                                        </span>
                                                    </div>

                                                    <h5 class="card-title">
                                                        {item.title.clone()}
                                                    </h5>

                                                    <p class="card-text">
                                                        {item.description.clone()}
                                                    </p>

                                                    <p class="text-muted small mb-0">
                                                        {format!("Date: {}", item.event_date)}
                                                    </p>

                                                    {
                                                        match item.status {
                                                            ListingStatus::PendingReview => html! {
                                                                <p class="text-muted small mt-3 mb-0">
                                                                    {"Your listing is waiting for moderator approval. Suggested matches will appear after approval."}
                                                                </p>
                                                            },

                                                            ListingStatus::Active | ListingStatus::Matched => html! {
                                                                <ListingMatches listing_id={item.id} />
                                                            },

                                                            _ => Html::default(),
                                                        }
                                                    }
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Html>()
                            }
                        </div>
                    },
                }
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ListingMatchesProps {
    listing_id: Uuid,
}

#[function_component(ListingMatches)]
fn listing_matches(props: &ListingMatchesProps) -> Html {
    let matches = use_state(|| {
        None::<Result<Vec<(MatchDto, Option<ListingDto>)>, String>>
    });
    let selected_match = use_state(|| None::<Uuid>);
    let confirming_match = use_state(|| None::<Uuid>);
    let action_message = use_state(|| None::<(bool, String)>);
    let confirmed_match = use_state(|| None::<Uuid>);
    let rejected_match = use_state(|| None::<Uuid>);
    let match_contacts = use_state(|| None::<MatchContactsResponse>);
    let show_contacts = use_state(|| false);
    let loading_contacts = use_state(|| false);
    let contact_error = use_state(|| None::<String>);

    {
        let matches = matches.clone();
        let listing_id = props.listing_id;

        use_effect_with(listing_id, move |listing_id| {
            let listing_id = *listing_id;

            spawn_local(async move {
                let result = async {
                    let token: String = LocalStorage::get("campuslost_token")
                        .map_err(|error| error.to_string())?;

                    let response = Request::get(&format!(
                        "{API_BASE}/listings/{listing_id}/matches"
                    ))
                        .header("Authorization", &format!("Bearer {token}"))
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;

                    if !response.ok() {
                        return Err(format!(
                            "Server returned status {}",
                            response.status()
                        ));
                    }

                    let match_items = response
                        .json::<Vec<MatchDto>>()
                        .await
                        .map_err(|error| error.to_string())?;

                    // Retrieve public listings to find the details of each match.
                    let listings_response = Request::get(&format!("{API_BASE}/listings"))
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;

                    if !listings_response.ok() {
                        return Err(format!(
                            "Could not retrieve matched listing details: {}",
                            listings_response.status()
                        ));
                    }

                    let public_listings = listings_response
                        .json::<Vec<ListingDto>>()
                        .await
                        .map_err(|error| error.to_string())?;

                    let detailed_matches = match_items
                        .into_iter()
                        .map(|item| {
                            let other_listing_id =
                                if item.lost_listing_id == listing_id {
                                    item.found_listing_id
                                } else {
                                    item.lost_listing_id
                                };

                            let other_listing = public_listings
                                .iter()
                                .find(|listing| listing.id == other_listing_id)
                                .cloned();

                            (item, other_listing)
                        })
                        .collect::<Vec<_>>();

                    Ok(detailed_matches)
                }
                    .await;

                matches.set(Some(result));
            });

            || ()
        });
    }

    html! {
        <div class="mt-3">
            <h6>{"Suggested matches"}</h6>
            {
                if let Some((is_error, message)) = &*action_message {
                    html! {
                        <div class={if *is_error {
                            "alert alert-danger py-2 small"
                        } else {
                            "alert alert-success py-2 small"
                        }}>
                            {message.clone()}
                        </div>
                    }
                } else {
                    Html::default()
                }
            }
            {
                match &*matches {
                    None => html! {
                        <p class="text-muted small mb-0">
                            {"Checking for matches..."}
                        </p>
                    },

                    Some(Err(error)) => html! {
                        <p class="text-danger small mb-0">
                            {format!("Could not load matches: {error}")}
                        </p>
                    },

                    Some(Ok(items)) if items.is_empty() => html! {
                        <p class="text-muted small mb-0">
                            {"No suggested matches found yet."}
                        </p>
                    },

                    Some(Ok(items)) => html! {
                        <div class="mt-2">
                            {
                                items.iter()
                                .filter(|(match_item, _)| {
                                    match_item.status != MatchStatus::Rejected
                                })
                                .map(|(match_item, other_listing)| {
                                    let is_selected = other_listing
                                        .as_ref()
                                        .map(|listing| *selected_match == Some(listing.id))
                                        .unwrap_or(false);

                                    let listing_id = other_listing.as_ref().map(|listing| listing.id);

                                    let selected_match_click = selected_match.clone();
                                    let confirming_match_state = confirming_match.clone();
                                    let action_message_state = action_message.clone();
                                    let confirmed_match_state = confirmed_match.clone();

                                    let match_id = match_item.id;
                                    let contacts_match_id = match_id;
                                    let confirm_click = Callback::from(move |_| {
                                        let confirming_match = confirming_match_state.clone();
                                        let action_message = action_message_state.clone();
                                        let confirmed_match = confirmed_match_state.clone();

                                        confirming_match.set(Some(match_id));
                                        action_message.set(None);

                                        spawn_local(async move {
                                            let result = async {
                                                let token: String = LocalStorage::get("campuslost_token")
                                                    .map_err(|error| error.to_string())?;

                                                let payload = MatchActionRequest {
                                                    action: MatchAction::Confirm,
                                                };

                                                let request = Request::post(&format!(
                                                    "{API_BASE}/matches/{match_id}/action"
                                                ))
                                                .header("Authorization", &format!("Bearer {token}"))
                                                .json(&payload)
                                                .map_err(|error| error.to_string())?;

                                                let response = request
                                                    .send()
                                                    .await
                                                    .map_err(|error| error.to_string())?;

                                                if !response.ok() {
                                                    let status = response.status();
                                                    let body = response.text().await.unwrap_or_default();

                                                    return Err(format!("Server returned {status}: {body}"));
                                                }

                                                let result = response
                                                    .json::<MatchActionResponse>()
                                                    .await
                                                    .map_err(|error| error.to_string())?;

                                                Ok(result)
                                            }
                                            .await;

                                            confirming_match.set(None);

                                            match result {
                                                Ok(response) => {
                                                    confirmed_match.set(Some(match_id));

                                                    let message = match response.return_id {
                                                        Some(return_id) => format!(
                                                            "Match confirmed! A return record was created ({return_id})."
                                                        ),
                                                        None => "Match confirmed successfully.".to_string(),
                                                    };

                                                    action_message.set(Some((false, message)));
                                                }
                                                Err(error) => {
                                                    action_message.set(Some((true, error)));
                                                }
                                            }
                                        });
                                    });
                                    let view_contacts_click = {
                                    let match_contacts = match_contacts.clone();
                                    let show_contacts = show_contacts.clone();
                                    let loading_contacts = loading_contacts.clone();
                                    let contact_error = contact_error.clone();

                                    Callback::from(move |_: web_sys::MouseEvent| {
                                        let match_contacts = match_contacts.clone();
                                        let show_contacts = show_contacts.clone();
                                        let loading_contacts = loading_contacts.clone();
                                        let contact_error = contact_error.clone();
                                        let match_id = contacts_match_id;

                                        spawn_local(async move {
                                            loading_contacts.set(true);
                                            contact_error.set(None);

                                            let token = match LocalStorage::get::<String>("campuslost_token") {
                                                Ok(token) => token,
                                                Err(_) => {
                                                    contact_error.set(Some("You are not logged in.".to_string()));
                                                    loading_contacts.set(false);
                                                    return;
                                                }
                                            };

                                            let response = Request::get(&format!(
                                                "{API_BASE}/matches/{match_id}/contacts"
                                            ))
                                            .header("Authorization", &format!("Bearer {token}"))
                                            .send()
                                            .await;

                                            match response {
                                                Ok(response) if response.ok() => {
                                                    match response.json::<MatchContactsResponse>().await {
                                                        Ok(data) => {
                                                            match_contacts.set(Some(data));
                                                            show_contacts.set(true);
                                                        }
                                                        Err(error) => {
                                                            contact_error.set(Some(format!(
                                                                "Could not read contact details: {error}"
                                                            )));
                                                        }
                                                    }
                                                }
                                                Ok(response) => {
                                                    contact_error.set(Some(format!(
                                                        "Could not load contact details. Server returned: {}",
                                                        response.status()
                                                    )));
                                                }
                                                Err(error) => {
                                                    contact_error.set(Some(format!(
                                                        "Request failed: {error}"
                                                    )));
                                                }
                                            }

                                            loading_contacts.set(false);
                                        });
                                    })
                                };
                                    let rejected_match_state = rejected_match.clone();
                                    let confirming_match_state = confirming_match.clone();
                                    let action_message_state = action_message.clone();

                                    let reject_click = Callback::from(move |_| {
                                        let rejected_match = rejected_match_state.clone();
                                        let confirming_match = confirming_match_state.clone();
                                        let action_message = action_message_state.clone();

                                        confirming_match.set(Some(match_id));
                                        action_message.set(None);

                                        spawn_local(async move {
                                            let result = async {
                                                let token: String = LocalStorage::get("campuslost_token")
                                                    .map_err(|error| error.to_string())?;

                                                let payload = MatchActionRequest {
                                                    action: MatchAction::Reject,
                                                };

                                                let request = Request::post(&format!(
                                                    "{API_BASE}/matches/{match_id}/action"
                                                ))
                                                .header("Authorization", &format!("Bearer {token}"))
                                                .json(&payload)
                                                .map_err(|error| error.to_string())?;

                                                let response = request
                                                    .send()
                                                    .await
                                                    .map_err(|error| error.to_string())?;

                                                if !response.ok() {
                                                    let status = response.status();
                                                    let body = response.text().await.unwrap_or_default();

                                                    return Err(format!("Server returned {status}: {body}"));
                                                }

                                                response
                                                    .json::<MatchActionResponse>()
                                                    .await
                                                    .map_err(|error| error.to_string())
                                            }
                                            .await;

                                            confirming_match.set(None);

                                            match result {
                                                Ok(_) => {
                                                    rejected_match.set(Some(match_id));
                                                    action_message.set(Some((
                                                        false,
                                                        "Match rejected.".to_string()
                                                    )));
                                                }
                                                Err(error) => {
                                                    action_message.set(Some((true, error)));
                                                }
                                            }
                                        });
                                    });
                                    html! {
                                        <div class="card mb-2">
                                            <button
                                                type="button"
                                                class="btn btn-link text-start text-decoration-none text-body p-3 w-100"
                                                onclick={Callback::from(move |_| {
                                                    if let Some(id) = listing_id {
                                                        if *selected_match_click == Some(id) {
                                                            selected_match_click.set(None);
                                                        } else {
                                                            selected_match_click.set(Some(id));
                                                        }
                                                    }
                                                })}
                                                disabled={other_listing.is_none()}
                                            >
                                                <div class="d-flex justify-content-between align-items-center gap-2">
                                                    <strong>
                                                        {
                                                            other_listing
                                                                .as_ref()
                                                                .map(|listing| listing.title.clone())
                                                                .unwrap_or_else(|| "Listing details unavailable".to_string())
                                                        }
                                                    </strong>

                                                    <span class="badge text-bg-primary">
                                                        {format!("{:.0}% match", match_item.score)}
                                                    </span>
                                                </div>

                                                <small class="text-muted">
                                                    {
                                                        if is_selected {
                                                            "Click to hide details"
                                                        } else {
                                                            "Click to view details"
                                                        }
                                                    }
                                                </small>
                                            </button>

                                            {
                                                if is_selected {
                                                    if let Some(listing) = other_listing.as_ref() {
                                                        html! {
                                                            <div class="card-body border-top">
                                                                <p>{listing.description.clone()}</p>

                                                                <p class="small text-muted mb-1">
                                                                    {format!("Date: {}", listing.event_date)}
                                                                </p>

                                                                <p class="small text-muted mb-0">
                                                                    {
                                                                        listing.location_detail
                                                                            .as_deref()
                                                                            .unwrap_or("No additional location details")
                                                                    }
                                                                </p>
                                                                <div class="mt-3 d-flex gap-2 flex-wrap">
                                                                    {
                                                                        if match_item.status == MatchStatus::Confirmed
                                                                            || *confirmed_match == Some(match_item.id) {
                                                                            html! {
                                                                                <button
                                                                                    class="btn btn-outline-primary btn-sm"
                                                                                    onclick={view_contacts_click.clone()}
                                                                                    disabled={*loading_contacts}
                                                                                >
                                                                                    {
                                                                                        if *loading_contacts {
                                                                                            "Loading contact details..."
                                                                                        } else {
                                                                                            "View contact details"
                                                                                        }
                                                                                    }
                                                                                </button>
                                                                            }
                                                                        } else if *rejected_match == Some(match_item.id) {
                                                                            html! {
                                                                                <button class="btn btn-secondary btn-sm" disabled=true>
                                                                                    {"Match rejected"}
                                                                                </button>
                                                                            }
                                                                        } else {
                                                                            html! {
                                                                                <>
                                                                                    <button
                                                                                        type="button"
                                                                                        class="btn btn-primary btn-sm"
                                                                                        onclick={confirm_click}
                                                                                        disabled={*confirming_match == Some(match_item.id)}
                                                                                    >
                                                                                        {
                                                                                            if *confirming_match == Some(match_item.id) {
                                                                                                "Processing..."
                                                                                            } else {
                                                                                                "Confirm match"
                                                                                            }
                                                                                        }
                                                                                    </button>

                                                                                    <button
                                                                                        type="button"
                                                                                        class="btn btn-outline-danger btn-sm"
                                                                                        onclick={reject_click}
                                                                                        disabled={*confirming_match == Some(match_item.id)}
                                                                                    >
                                                                                        {"Reject match"}
                                                                                    </button>
                                                                                </>
                                                                            }
                                                                        }
                                                                    }
                                                                </div>
                                                            </div>
                                                        }
                                                    } else {
                                                        Html::default()
                                                    }
                                                } else {
                                                    Html::default()
                                                }
                                            }
                                        </div>
                                    }
                                }).collect::<Html>()
                            }
                        </div>
                    },
                }
            }
            {
                if *show_contacts {
                    if let Some(data) = (*match_contacts).as_ref() {
                        html! {
                            <>
                                <div
                                    class="modal-backdrop fade show"
                                    onclick={{
                                        let show_contacts = show_contacts.clone();
                                        Callback::from(move |_: web_sys::MouseEvent| {
                                            show_contacts.set(false);
                                        })
                                    }}
                                />

                                <div
                                    class="modal d-block"
                                    tabindex="-1"
                                    role="dialog"
                                >
                                    <div class="modal-dialog modal-dialog-centered">
                                        <div class="modal-content">
                                            <div class="modal-header">
                                                <h5 class="modal-title">
                                                    {"Contact details"}
                                                </h5>
                                                <button
                                                    type="button"
                                                    class="btn-close"
                                                    aria-label="Close"
                                                    onclick={{
                                                        let show_contacts = show_contacts.clone();
                                                        Callback::from(move |_: web_sys::MouseEvent| {
                                                            show_contacts.set(false);
                                                        })
                                                    }}
                                                />
                                            </div>

                                            <div class="modal-body">
                                                <p>
                                                    <strong>{"Name: "}</strong>
                                                    {data.contact.full_name.clone()}
                                                </p>

                                                <p>
                                                    <strong>{"Email: "}</strong>
                                                    <a href={format!("mailto:{}", data.contact.email)}>
                                                        {data.contact.email.clone()}
                                                    </a>
                                                </p>

                                                {
                                                    if let Some(phone) = data.contact.phone.as_ref() {
                                                        html! {
                                                            <p>
                                                                <strong>{"Phone: "}</strong>
                                                                {phone.clone()}
                                                            </p>
                                                        }
                                                    } else {
                                                        Html::default()
                                                    }
                                                }

                                                {
                                                    if let Some(telegram) = data.contact.telegram_handle.as_ref() {
                                                        html! {
                                                            <p>
                                                                <strong>{"Telegram: "}</strong>
                                                                {telegram.clone()}
                                                            </p>
                                                        }
                                                    } else {
                                                        Html::default()
                                                    }
                                                }

                                                {
                                                    if let Some(vk) = data.contact.vk_handle.as_ref() {
                                                        html! {
                                                            <p>
                                                                <strong>{"VK: "}</strong>
                                                                {vk.clone()}
                                                            </p>
                                                        }
                                                    } else {
                                                        Html::default()
                                                    }
                                                }

                                                <p class="mb-0">
                                                    <strong>{"Preferred contact: "}</strong>
                                                    {format!("{:?}", data.contact.preferred_contact)}
                                                </p>
                                            </div>

                                            <div class="modal-footer">
                                                <button
                                                    type="button"
                                                    class="btn btn-secondary"
                                                    onclick={{
                                                        let show_contacts = show_contacts.clone();
                                                        Callback::from(move |_: web_sys::MouseEvent| {
                                                            show_contacts.set(false);
                                                        })
                                                    }}
                                                >
                                                    {"Close"}
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </>
                        }
                    } else {
                        Html::default()
                    }
                } else {
                    Html::default()
                }
            }
        </div>
    }
}

#[function_component(App)]
fn app() -> Html {
    let show_register = use_state(|| false);
    let is_logged_in = use_state(|| false);
    let current_page = use_state(|| "home".to_string());
    let email_ref = use_node_ref();
    let password_ref = use_node_ref();
    let login_message = use_state(|| None::<(bool, String)>);

    let full_name_ref = use_node_ref();
    let register_email_ref = use_node_ref();
    let register_password_ref = use_node_ref();
    let register_message = use_state(|| None::<String>);

    let toggle_auth = {
        let show_register = show_register.clone();

        Callback::from(move |_| {
            show_register.set(!*show_register);
        })
    };

    // Login request
    let login = {
        let email_ref = email_ref.clone();
        let password_ref = password_ref.clone();
        let login_message = login_message.clone();
        let is_logged_in = is_logged_in.clone();

        Callback::from(move |_: MouseEvent| {
            let Some(email_input) = email_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(password_input) = password_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let payload = LoginRequest {
                email: email_input.value(),
                password: password_input.value(),
            };

            let login_message = login_message.clone();
            let is_logged_in = is_logged_in.clone();

            spawn_local(async move {
                login_message.set(None);

                let request = Request::post(&format!("{API_BASE}/auth/login"))
                    .json(&payload);

                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not create request: {error}"),
                        )));
                        return;
                    }
                };

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not connect to server: {error}"),
                        )));
                        return;
                    }
                };

                let status = response.status();
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not read server response: {error}"),
                        )));
                        return;
                    }
                };

                if !(200..300).contains(&status) {
                    let message = serde_json::from_str::<ApiError>(&body)
                        .map(|error| error.message)
                        .unwrap_or_else(|_| {
                            format!("Login failed (HTTP {status}): {body}")
                        });

                    login_message.set(Some((true, message)));
                    return;
                }

                let auth = match serde_json::from_str::<AuthResponse>(&body) {
                    Ok(auth) => auth,
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Invalid server response: {error}"),
                        )));
                        return;
                    }
                };

                match LocalStorage::set("campuslost_token", auth.access_token) {
                    Ok(()) => {
                        login_message.set(Some((
                            false,
                            "Login successful! JWT token saved.".to_string(),
                        )));
                        is_logged_in.set(true);
                    }
                    Err(error) => {
                        login_message.set(Some((
                            true,
                            format!("Could not save login token: {error}"),
                        )));
                    }
                }
            });
        })
    };

    // Registration request
    let register = {
        let full_name_ref = full_name_ref.clone();
        let email_ref = register_email_ref.clone();
        let password_ref = register_password_ref.clone();
        let register_message = register_message.clone();
        let login_message = login_message.clone();
        let show_register = show_register.clone();

        Callback::from(move |_: MouseEvent| {
            let Some(full_name) = full_name_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(email) = email_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let Some(password) = password_ref.cast::<HtmlInputElement>() else {
                return;
            };

            let payload = RegisterRequest {
                full_name: full_name.value(),
                email: email.value(),
                password: password.value(),
                phone: None,
                telegram_handle: None,
                vk_handle: None,
                preferred_contact: None,
            };

            let register_message = register_message.clone();
            let login_message = login_message.clone();
            let show_register = show_register.clone();

            spawn_local(async move {
                register_message.set(None);

                let request = Request::post(&format!("{API_BASE}/auth/register"))
                    .json(&payload);

                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        register_message.set(Some(format!(
                            "Could not create request: {error}"
                        )));
                        return;
                    }
                };

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        register_message.set(Some(format!(
                            "Could not connect to server: {error}"
                        )));
                        return;
                    }
                };

                let status = response.status();

                if !(200..300).contains(&status) {
                    let body = response.text().await.unwrap_or_default();

                    let message = serde_json::from_str::<ApiError>(&body)
                        .map(|error| error.message)
                        .unwrap_or_else(|_| {
                            format!("Registration failed (HTTP {status}): {body}")
                        });

                    register_message.set(Some(message));
                    return;
                }

                show_register.set(false);
                login_message.set(Some((
                    false,
                    "Account created! You can now sign in.".to_string(),
                )));
            });
        })
    };

    html! {
        if *is_logged_in {
            <>
                <nav class="navbar navbar-expand-lg navbar-dark bg-primary shadow-sm">
                    <div class="container">
                        <a class="navbar-brand fw-bold" href="/">
                            {"CampusLost"}
                        </a>

                        <div class="navbar-nav ms-auto flex-row gap-2">
                            <button
                                type="button"
                                class={if *current_page == "home" {
                                    "btn btn-light btn-sm"
                                } else {
                                    "btn btn-outline-light btn-sm"
                                }}
                                onclick={{
                                    let current_page = current_page.clone();
                                    Callback::from(move |_| {
                                        current_page.set("home".to_string());
                                    })
                                }}
                            >
                                {"Browse"}
                            </button>

                            <button
                                type="button"
                                class={if *current_page == "my_listings" {
                                    "btn btn-light btn-sm"
                                } else {
                                    "btn btn-outline-light btn-sm"
                                }}
                                onclick={{
                                    let current_page = current_page.clone();
                                    Callback::from(move |_| {
                                        current_page.set("my_listings".to_string());
                                    })
                                }}
                            >
                                {"My Listings"}
                            </button>
                        </div>
                    </div>
                </nav>

                {
                    if *current_page == "home" {
                        html! { <ListingsPage /> }
                    } else {
                        html! { <MyListingsPage /> }
                    }
                }
            </>
        } else {
            <div class="min-vh-100 bg-light d-flex flex-column">
                <nav class="navbar navbar-dark bg-primary shadow-sm">
                    <div class="container">
                        <a class="navbar-brand fw-bold" href="/">
                            {"CampusLost"}
                        </a>
                    </div>
                </nav>

                <main class="container flex-grow-1 d-flex justify-content-center align-items-center py-5">
                    <div class="card shadow-sm border-0 w-100" style="max-width: 450px;">
                        <div class="card-body p-4 p-md-5">
                            <h2 class="text-center fw-bold mb-2">
                                {
                                    if *show_register {
                                        "Create an account"
                                    } else {
                                        "Welcome back"
                                    }
                                }
                            </h2>

                            <p class="text-center text-muted mb-4">
                                {
                                    if *show_register {
                                        "Register to start using CampusLost."
                                    } else {
                                        "Sign in to your CampusLost account."
                                    }
                                }
                            </p>

                            {
                                if *show_register {
                                    html! {
                                        <div>
                                            <div class="mb-3">
                                                <label for="full_name" class="form-label fw-semibold">
                                                    {"Full name"}
                                                </label>
                                                <input
                                                    type="text"
                                                    id="full_name"
                                                    class="form-control"
                                                    placeholder="John Smith"
                                                    ref={full_name_ref.clone()}
                                                />
                                            </div>

                                            <div class="mb-3">
                                                <label for="register_email" class="form-label fw-semibold">
                                                    {"Email address"}
                                                </label>
                                                <input
                                                    type="email"
                                                    id="register_email"
                                                    class="form-control"
                                                    placeholder="name@example.com"
                                                    ref={register_email_ref.clone()}
                                                />
                                            </div>

                                            <div class="mb-4">
                                                <label for="register_password" class="form-label fw-semibold">
                                                    {"Password"}
                                                </label>
                                                <input
                                                    type="password"
                                                    id="register_password"
                                                    class="form-control"
                                                    placeholder="Create a password"
                                                    ref={register_password_ref.clone()}
                                                />
                                            </div>

                                            <button
                                                type="button"
                                                class="btn btn-primary w-100 py-2"
                                                onclick={register}
                                            >
                                                {"Create account"}
                                            </button>

                                            {
                                                if let Some(message) = &*register_message {
                                                    html! {
                                                        <div class="alert alert-danger mt-3" role="alert">
                                                            {message.clone()}
                                                        </div>
                                                    }
                                                } else {
                                                    Html::default()
                                                }
                                            }
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <div>
                                            <div class="mb-3">
                                                <label for="login_email" class="form-label fw-semibold">
                                                    {"Email address"}
                                                </label>
                                                <input
                                                    type="email"
                                                    id="login_email"
                                                    class="form-control"
                                                    placeholder="name@example.com"
                                                    ref={email_ref.clone()}
                                                />
                                            </div>

                                            <div class="mb-4">
                                                <label for="login_password" class="form-label fw-semibold">
                                                    {"Password"}
                                                </label>
                                                <input
                                                    type="password"
                                                    id="login_password"
                                                    class="form-control"
                                                    placeholder="Enter your password"
                                                    ref={password_ref.clone()}
                                                />
                                            </div>

                                            <button
                                                type="button"
                                                class="btn btn-primary w-100 py-2"
                                                onclick={login}
                                            >
                                                {"Sign in"}
                                            </button>

                                            {
                                                if let Some((is_error, message)) = &*login_message {
                                                    html! {
                                                        <div
                                                            class={if *is_error {
                                                                "alert alert-danger mt-3"
                                                            } else {
                                                                "alert alert-success mt-3"
                                                            }}
                                                            role="alert"
                                                        >
                                                            {message.clone()}
                                                        </div>
                                                    }
                                                } else {
                                                    Html::default()
                                                }
                                            }
                                        </div>
                                    }
                                }
                            }

                            <hr class="my-4" />

                            <div class="text-center">
                                <span class="text-muted">
                                    {
                                        if *show_register {
                                            "Already have an account? "
                                        } else {
                                            "Don't have an account? "
                                        }
                                    }
                                </span>

                                <button
                                    type="button"
                                    class="btn btn-link p-0 text-decoration-none"
                                    onclick={toggle_auth}
                                >
                                    {
                                        if *show_register {
                                            "Sign in"
                                        } else {
                                            "Register"
                                        }
                                    }
                                </button>
                            </div>
                        </div>
                    </div>
                </main>

                <footer class="bg-white border-top py-3">
                    <div class="container text-center text-muted small">
                        {"CampusLost © 2026"}
                    </div>
                </footer>
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
