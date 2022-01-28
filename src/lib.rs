use reqwest::blocking::Client;
use std::fmt;
use serde_json::Value;

#[derive(Debug)]
pub struct Error {
    pub id: i32,
    pub description: Option<String>,
    pub info: Option<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error id: {},\n Error description: {:?},\n Addition info: {:?}", self.id, self.description, self.info)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn from_json(json: &Value) -> Self {
        Self {
            id: json.get("id").unwrap().as_i64().unwrap() as i32,
            description: Some(json.get("description").unwrap().as_str().unwrap().to_string()),
            info: Some(json.get("info").unwrap().as_str().unwrap().to_string()),
        }
    }

    pub fn new(id: i32, description: Option<String>, info: Option<String>) -> Self {
        Self {
            id,
            description,
            info
        }
    }
}


#[derive(Clone)]
pub enum State {
    Disconnected(String),
    Registration,
    Idle,
    Searching,
    Playing,
}
impl State {
    pub fn from_id(state_id: u64) -> State {
        match state_id {
            0 => State::Registration,
            1 => State::Idle,
            2 => State::Searching,
            3 => State::Playing,
            _ => State::Disconnected("Unknown state id".into()),
        }
    }
}

pub struct Player {
    pub nickname: String,
    pub id: u64,
}

impl Player {
    pub fn new(nickname: &str, id: u64) -> Self {
        Self {
            nickname: nickname.into(),
            id,
        }
    }
}

#[allow(dead_code)]
struct RegPlayerInfo {
    nickname: String,
    id: u64,
    player_id: u64,
}

impl RegPlayerInfo {
    fn new(nickname: &str, id: u64, player_id: u64) -> Self {
        Self {
            nickname: nickname.into(),
            id, player_id,
        }
    }
}

pub struct Connection {
    state: State,
    info: Option<RegPlayerInfo>,
    base_url: String,
    client: Client,
}

#[allow(dead_code)]
impl Connection {
    fn parse(resp: &Value) -> Result<&Value, Error> {
        let resp = resp.as_object().unwrap();
        if resp.contains_key("success") {
            Ok(resp.get("success").unwrap())
        }
        else {
            Err(Error::from_json(resp.get("error").unwrap()))
        }
    }

    fn get(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.client.get(format!("{}/{command}", self.base_url.as_str())).query(query).body(body.to_string()).send()?.json()?;
        Ok(resp)
    }
    fn get_parsed(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let x = self.get(command, query, body)?;
        Ok(Connection::parse(&x)?.clone())
    }

    fn get_with_id(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        self.get(format!("/{}/{command}", self.info.as_ref().unwrap().id).as_str(), query, body)
    }
    fn get_parsed_with_id(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        self.get_parsed(format!("/{}/{command}", self.info.as_ref().unwrap().id).as_str(), query, body)
    }


    fn post(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.client.post(format!("{}/{command}", self.base_url.as_str())).query(query).body(body.to_string()).send()?.json()?;
        Ok(resp)
    }
    fn post_parsed(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let x = self.post(command, query, body)?;
        Ok(Connection::parse(&x)?.clone())
    }

    fn post_with_id(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        self.post(format!("/{}/{command}", self.info.as_ref().unwrap().id).as_str(), query, body)
    }
    fn post_parsed_with_id(&self, command: &str, query: &[(&str, &str)], body: &str) -> Result<Value, Box<dyn std::error::Error>> {
        self.post_parsed(format!("/{}/{command}", self.info.as_ref().unwrap().id).as_str(), query, body)
    }
}

impl Connection {
    pub fn new(url: &str) -> Self {
        Self {
            state: State::Registration,
            info: None,
            base_url: url.into(),
            client: Client::new(),
        }
    }

    pub fn get_error_description(&self, error_id: i32) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.get_parsed("error_description", &[("id", error_id.to_string().as_str())], "")?
               .as_object().unwrap()
               .get("description").unwrap()
               .as_str().unwrap().to_string())
    }

    pub fn get_players(&self) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
        let mut res = vec![];

        let x = self.get_parsed("players", &[], "")?;
        let y = x.as_object().unwrap().get("players").unwrap().as_array().unwrap();
        for z in y {
            let s = z.as_str().unwrap();
            let colon_pos = s.find(':').unwrap();
            let id = s[..colon_pos].parse().unwrap();
            let nickname = &s[colon_pos + 1..];

            res.push(Player::new(nickname, id));
        }
        
        Ok(res)
    }

    pub fn get_nickname(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.info.as_ref().unwrap().nickname.clone())
    }

    pub fn get_state(&mut self) -> Result<State, Box<dyn std::error::Error>> {
        let state_id = self.get_parsed_with_id("state", &[], "")?
                                  .as_object().unwrap()
                                  .get("state").unwrap()
                                  .as_u64().unwrap();
        let state = State::from_id(state_id);
        if let State::Disconnected(_) = state {

        }
        self.state = state.clone();
        Ok(state)
    }
    pub fn get_stored_state(&self) -> State {
        self.state.clone()
    }

    pub fn register(&mut self, nickname: String) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self.post_parsed("register", &[("name", nickname.as_str())], "")?;
        let pl =  resp.as_object().unwrap()
                                        .get("player").unwrap()
                                        .as_object().unwrap();

        let nickname = pl.get("nickname").unwrap().as_str().unwrap();
        let id = pl.get("id").unwrap().as_u64().unwrap();
        let player_id = pl.get("player_id").unwrap().as_u64().unwrap();
        
        self.info = Some(RegPlayerInfo::new(nickname, id, player_id));
        self.state = State::Idle;

        Ok(())
    }

    pub fn search(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.post_parsed_with_id("search", &[], "")?;
        self.state = State::Searching;

        Ok(())
    }
    pub fn idle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.post_parsed_with_id("idle", &[], "")?;
        self.state = State::Idle;

        Ok(())
    }

    pub fn send_request(&mut self, send_to: u64) -> Result<(), Box<dyn std::error::Error>> {
        let x = self.post_parsed_with_id("requests", &[("send_to", send_to.to_string().as_str())], "")?;
        let in_game = x.as_object().unwrap().get("in_game").unwrap().as_bool().unwrap();

        if in_game {
            self.state = State::Playing;
        }
        Ok(())
    }
    pub fn get_requests(&mut self) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
        let mut res = vec![];

        let x = self.get_parsed_with_id("requests", &[], "")?;
        let y = x.as_object().unwrap().get("requests").unwrap().as_array().unwrap();
        for z in y {
            let s = z.as_str().unwrap();
            let colon_pos = s.find(':').unwrap();
            let id = s[..colon_pos].parse().unwrap();
            let nickname = &s[colon_pos + 1..];

            res.push(Player::new(nickname, id));
        }
        
        Ok(res)
    }

    pub fn send_message(&mut self, message: String) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.post_parsed_with_id("messages", &[], message.as_str())?;

        Ok(())
    }
    pub fn get_messages(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut res = vec![];

        let x = self.get_parsed_with_id("messages", &[], "")?;
        let y = x.as_object().unwrap().get("messages").unwrap().as_array().unwrap();

        for z in y {
            let s = z.as_str().unwrap();
            res.push(s.to_string());
        }

        Ok(res)
    }

    pub fn end_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.post_parsed_with_id("end_game", &[], "")?;
        Ok(())
    }
}