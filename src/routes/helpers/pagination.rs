use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationQuery {
    /// Page number; defaults to 1. Zero is treated as 1.
    page: Option<u32>,
    /// Page size; defaults to 20 and is clamped to 1–100.
    per_page: Option<u32>,
}

impl PaginationQuery {
    pub fn normalize(&self) -> Pagination {
        Pagination {
            page: self.page.unwrap_or(1).max(1),
            per_page: self.per_page.unwrap_or(20).clamp(1, 100),
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Pagination {
    page: u32,
    per_page: u32,
}

impl Pagination {
    pub fn limit_offset(&self) -> (i64, i64) {
        let limit = i64::from(self.per_page);
        // Widen before multiplying: even u32::MAX pages at 100 rows fit in i64.
        let offset = i64::from(self.page - 1) * limit;
        (limit, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::PaginationQuery;
    use actix_web::web::Query;
    use serde_json::json;

    #[test]
    fn pagination_defaults_and_boundaries_produce_matching_metadata_and_sql() {
        for (query, expected_page, expected_size, expected_offset) in [
            ("", 1, 20, 0),
            ("page=2", 2, 20, 20),
            ("per_page=2", 1, 2, 0),
            ("page=0&per_page=0", 1, 1, 0),
            ("page=2&per_page=2", 2, 2, 2),
            ("page=2&per_page=100", 2, 100, 100),
            ("page=2&per_page=101", 2, 100, 100),
            ("page=2&per_page=4294967295", 2, 100, 100),
            (
                "page=4294967295&per_page=100",
                u32::MAX,
                100,
                429_496_729_400,
            ),
        ] {
            let pagination = Query::<PaginationQuery>::from_query(query)
                .unwrap()
                .normalize();
            assert_eq!(
                serde_json::to_value(&pagination).unwrap(),
                json!({"page": expected_page, "per_page": expected_size}),
                "{query}"
            );
            assert_eq!(
                pagination.limit_offset(),
                (i64::from(expected_size), expected_offset),
                "{query}"
            );
        }
    }

    #[test]
    fn invalid_pagination_queries_are_rejected() {
        for parameter in ["page", "per_page"] {
            for value in ["-1", "4294967296", "abc", "1.5", ""] {
                let query = format!("{parameter}={value}");
                assert!(
                    Query::<PaginationQuery>::from_query(&query).is_err(),
                    "{query}"
                );
            }
            let query = format!("{parameter}=1&{parameter}=2");
            assert!(
                Query::<PaginationQuery>::from_query(&query).is_err(),
                "{query}"
            );
        }
    }
}
