use crate::error::Result;
use crate::server::{AppData, Authentication, CommonRequest, CommonResponse, PooledConnection};
use crate::sql::internal::problem_list_manager::ProblemListManager;

use serde::{Deserialize, Serialize};
use tide::{Request, Response};

pub(crate) async fn get_list<A: Clone + Authentication>(request: Request<AppData<A>>) -> Response {
    fn unpack_request<A>(request: Request<AppData<A>>) -> Result<(String, PooledConnection)> {
        let token = request.get_cookie("token")?;
        let conn = request.state().pool.get()?;
        Ok((token, conn))
    }
    fn construct_response(conn: PooledConnection, internal_user_id: String) -> Result<Response> {
        let list = conn.get_list(&internal_user_id)?;
        let response = Response::ok().body_json(&list)?;
        Ok(response)
    }
    let client = request.state().authentication.clone();
    match unpack_request(request) {
        Ok((token, conn)) => match client.get_user_id(&token).await {
            Ok(internal_user_id) => match construct_response(conn, internal_user_id) {
                Ok(response) => response,
                _ => Response::internal_error(),
            },
            _ => Response::bad_request(),
        },
        Err(_) => Response::bad_request(),
    }
}

pub(crate) async fn create_list<A: Authentication + Clone>(
    request: Request<AppData<A>>,
) -> Response {
    #[derive(Deserialize)]
    struct Query {
        list_name: String,
    }
    #[derive(Serialize)]
    struct Created {
        internal_list_id: String,
    }
    async fn unpack_request<A>(
        mut request: Request<AppData<A>>,
    ) -> Result<(String, PooledConnection, String)> {
        let query: Query = request.body_json().await?;
        let token = request.get_cookie("token")?;
        let conn = request.state().pool.get()?;
        Ok((token, conn, query.list_name))
    }
    fn create_response(conn: PooledConnection, user_id: &str, list_name: &str) -> Result<Response> {
        let internal_list_id = conn.create_list(user_id, list_name)?;
        let created = Created { internal_list_id };
        let response = Response::ok().body_json(&created)?;
        Ok(response)
    }

    use log::info;
    let client = request.state().authentication.clone();
    match unpack_request(request).await {
        Ok((token, conn, list_name)) => match client.get_user_id(&token).await {
            Ok(internal_user_id) => match create_response(conn, &internal_user_id, &list_name) {
                Ok(response) => response,
                Err(e) => {
                    info!("{:?}", e);
                    Response::bad_request()
                }
            },
            Err(e) => {
                info!("{:?}", e);
                Response::bad_request()
            }
        },
        Err(e) => {
            info!("{:?}", e);
            Response::bad_request()
        }
    }
}
