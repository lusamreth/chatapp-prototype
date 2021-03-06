use actix::{Message, Recipient};
use std::collections::HashMap;
//use std::sync::Arc;
use pbkdf2;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
pub mod jwt;
pub mod utility;

//type Clients = Arc<HashMap<Uuid,client>>;
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct Payload {
    pub(crate) message: String,
    pub date: Duration,
}

type MessageStorage = HashMap<Uuid, Payload>;
impl Payload {
    pub fn new(msg: &str) -> Self {
        Payload {
            message: msg.to_string(),
            date: utility::timestamp_now(),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    pub client_id: Uuid,
    // use to send message to actix web actor
    pub address: Option<Recipient<Payload>>,
    pub(crate) user: User,
}

#[derive(Debug, Clone)]
pub struct User {
    pub uid: Uuid,
    pub username: String,
    created_at: Duration,
    pub token_version: usize,
    // contain hash
    pwd: String,
}

impl User {
    pub fn new(username: String, pwd: String) -> Self {
        let mut hash_buffer = [0u8; 64];
        //pbkdf2::pbkdf2(pwd.as_bytes(), "example!!".as_bytes(), 5, &mut hash_buffer);
        let a = pbkdf2::pbkdf2_simple(&pwd, 20).unwrap();
        // require atleast
        User {
            username,
            uid: Uuid::new_v4(),
            created_at: utility::timestamp_now(),
            //pwd: "a".to_string(),
            token_version: 0,
            pwd: a,
        }
    }
    pub fn comp_pass(&self, pass: &str) -> bool {
        utility::compare_sha256(pass, self.pwd.as_str())
    }
}

impl Client {
    pub fn new(user: User) -> Self {
        Client {
            client_id: Uuid::new_v4(),
            address: None,
            user,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub cl_id: Uuid,
    pub room_id: Uuid,
    pub message: String,
}
#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub room_id: Uuid,
    pub client_ids: Vec<Uuid>,
    pub capacity: Option<i32>,
    message_storage: MessageStorage,
    pub participants: usize,
    pub admin: Uuid,
}

impl Room {
    pub fn create(name: String, cap: Option<i32>, admin: Uuid) -> Result<Self, RoomError> {
        let num;
        if cap.is_some() {
            num = cap.unwrap();
        } else {
            num = -1;
        }

        if name.len() == 0 {
            return Err(RoomError::REFUSED(RoomRejection::Reject(
                RefusedReason::EMPTY,
            )));
        } else if num == 0 {
            return Err(RoomError::UNACCEPTABLE(String::from(
                "The capacity must be greater than 0!",
            )));
        } else {
            let mut new_cls = Vec::new();
            new_cls.push(admin.clone());
            Ok(Room {
                name,
                capacity: cap,
                message_storage: HashMap::new(),
                room_id: Uuid::new_v4(),
                client_ids: new_cls,
                participants: 1,
                admin,
            })
        }
    }

    pub fn append_client(&mut self, new_client: Uuid) {
        self.client_ids.push(new_client);
    }

    pub fn append_message(&mut self, payload: Payload, sender: Uuid) {
        self.message_storage.insert(sender.clone(), payload);
    }

    pub fn iter(&self) -> RoomIter {
        RoomIter {
            iter: self.client_ids.clone(),
        }
    }
}

pub struct RoomIter {
    pub iter: Vec<Uuid>,
}

impl Iterator for RoomIter {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.iter().next().map(|e| *e)
    }
}

// #[derive(Message)]
// #[rtype(result = "Result<RegistrationStatus,()>")]

use derive_more::{Display, Error};
#[derive(Message)]
#[rtype(result = "RegisterRes")]
pub struct Registration {
    pub username: String,
    pub password: String,
}
// #[rtype(result = "(RegistrationStatus,Option<Uuid>)")]
#[derive(Message)]
#[rtype(result = "Self")]
pub struct RegisterRes {
    pub status: RegistrationStatus,
    pub cl_id: Option<Uuid>,
}
#[derive(Debug, Display, Error)]
pub enum RefusedReason {
    #[display(fmt = "Bad formatting!")]
    BADFORMAT,
    #[display(fmt = "Empty!")]
    EMPTY,
}

type Field = String;
#[derive(Debug)]
pub enum RegistrationStatus {
    CREATED,
    REFUSED(RefusedReason, Field),
    FAILED(FailureReason),
}
#[derive(Debug, Display, Error)]
pub enum FailureReason {
    // database could be other error!
    #[display(fmt = "Poisioned While writing to storage")]
    ACCESSWRITEERROR,
    #[display(fmt = "Collided While appending to storage")]
    COLLISION,
    #[display(fmt = "Poisioned While reading to storage")]
    ACCESSREADERROR,
}

pub struct ListUser;

impl Message for ListUser {
    type Result = HashMap<Uuid, User>;
    //type Result = Vec<User>;
}
#[derive(Message, Debug)]
#[rtype("result = Self")]
pub struct RoomCreation {
    pub status: RoomCreationStatus,
    pub handle: Option<Uuid>,
}

#[derive(Debug)]
pub enum RoomError {
    UNACCEPTABLE(String),
    REFUSED(RoomRejection),
    INTERNALERROR(FailureReason),
}

#[derive(Debug)]
pub enum RoomCreationStatus {
    CREATED,
    ERROR(RoomError),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRoom {
    pub name: String,
    pub capacity: Option<i32>,
    pub creator: Uuid,
}

impl Message for CreateRoom {
    type Result = RoomCreation;
}

pub struct RoomRetrieval {}
impl Message for RoomRetrieval {
    type Result = Result<HashMap<Uuid, Room>, FailureReason>;
}
pub trait SignalController {
    fn connect(&mut self, user: Uuid, addr: &Recipient<Payload>, room_id: Uuid) -> SignalOutput;
    fn diconnect(&mut self, user: Uuid) -> SignalOutput;
    fn pending(&mut self, addr: Recipient<Payload>) -> SignalOutput;
}

pub enum SignalCode {
    Connect,
    Disconnect,
    Pending,
}
pub struct SignalInput {
    pub id: Uuid,
    // get this address from the websocket pipe!
    pub code: SignalCode,
    pub addr: Recipient<Payload>,
    pub room_id: Uuid,
}

impl Message for SignalInput {
    type Result = SignalOutput;
}

pub struct SignalOutput {
    pub status: ConnectionStatus,
    pub signaled_at: Duration,
}

#[derive(Debug, Display)]
pub enum AbortReason {
    #[display(fmt = "Failing internally during connection!")]
    Internal(FailureReason),
    #[display(fmt = "Failing during connection due to unprocessable entity")]
    External(RefusedReason),
    UNACCEPTABLE(RoomRejection),
}
#[derive(Debug, Display)]
pub enum ConnectionStatus {
    #[display(fmt = "Connected sucessfully")]
    Connected,
    #[display(fmt = "Aborted due to error!")]
    Aborted(AbortReason),
    #[display(fmt = "Disconnected successfully!")]
    Disconnected,
}

#[derive(Message)]
#[rtype(result = "JoinOutput")]
pub struct JoinInput {
    pub target_id: Uuid, // room to join,
    pub client_id: Uuid,
    pub addr: Recipient<Payload>,
}

pub const UNKNOWN_ROOM_MSG: &'static str = "The room with this particular id is not found !";
pub const UNKNOWN_USER_MSG: &'static str = "The user with this particular id is not found !";
#[derive(Debug, Display)]
pub enum RoomRejection {
    #[display(fmt = "{}", UNKNOWN_ROOM_MSG)]
    UnknownRoom,
    #[display(fmt = "{}", UNKNOWN_USER_MSG)]
    UnknownUser,
    #[display(fmt = "The Room join request has been rejected")]
    Reject(RefusedReason),
}

pub enum JoinOutput {
    Success,
    Rejected(RoomRejection),
    Failed(FailureReason),
}
<<<<<<< HEAD
=======
<<<<<<< HEAD
>>>>>>> d41459f (Improving authentication logic!)
pub enum LoginFailure {
    Internal(FailureReason),
    UserFailure,
}
pub enum LoginStatus {
    Passed,
    Failed(LoginFailure),
}
pub struct LoginRes {
    pub status: LoginStatus,
    pub cl_id: Option<Uuid>,
}

#[derive(Clone, Message)]
#[rtype(result = "LoginRes")]
<<<<<<< HEAD
=======
=======

#[derive(Clone, Message)]
#[rtype(result = "Option<Uuid>")]
>>>>>>> 21fb43b (Handshake authentication)
>>>>>>> d41459f (Improving authentication logic!)
pub struct LoginMessage {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Display)]
pub enum BearerFailure {
    #[display(fmt = "Invalid Token input!")]
    InvalidToken,
    #[display(fmt = "The header is empty!")]
    EmptyHeader,
    #[display(fmt = "Expired jwt token!")]
    ExpiredJwt,
    #[display(fmt = "Missing authorizing cookie!")]
    EmptyCookie,
    #[display(fmt = "Error while parsing the jwt cookie!")]
    ParsingError,
    #[display(fmt = "Jwt token contains irregular components!")]
    BadJwtComponent,
}
<<<<<<< HEAD
=======
<<<<<<< HEAD
=======

#[derive(Debug, Display)]
>>>>>>> 21fb43b (Handshake authentication)
>>>>>>> d41459f (Improving authentication logic!)
pub enum AuthStatus {
    Success,
    Fail(BearerFailure),
}

pub struct AuthorizationError {
    pub status: String,
    pub reason: String,
}
<<<<<<< HEAD
=======
<<<<<<< HEAD
=======

pub struct Rtoken(pub String, pub String);

impl Message for Rtoken {
    type Result = Result<String, AuthStatus>;
}
>>>>>>> 21fb43b (Handshake authentication)
>>>>>>> d41459f (Improving authentication logic!)
