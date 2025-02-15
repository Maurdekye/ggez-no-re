use std::{cell::RefCell, error::Error, rc::Rc, time::SystemTime};

use discord_sdk::{
    Discord, DiscordApp, Subscriptions,
    activity::{Activity, ActivityBuilder},
    user::User,
    wheel::{UserState, Wheel},
};
use log::{debug, error};
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct DiscordPresence(Rc<RefCell<DiscordInner>>);

impl DiscordPresence {
    pub fn new(application_id: i64) -> Result<DiscordPresence, Box<dyn Error>> {
        debug!("Initializing tokio runtime");
        let runtime = Runtime::new()?;

        let (discord, user, wheel) = runtime.block_on(async {
            debug!("Initializing wheel");
            let (wheel, handler) = Wheel::new(Box::new(|err| {
                error!("encountered an error creating discord wheel: {err}");
            }));

            let mut user = wheel.user();

            debug!("Initializing Discord");
            let discord = Discord::new(
                DiscordApp::PlainId(application_id),
                Subscriptions::empty(),
                Box::new(handler),
            )?;

            debug!("Waiting for handshake...");
            user.0.changed().await?;

            let user = match &*user.0.borrow() {
                UserState::Connected(user) => user.clone(),
                UserState::Disconnected(err) => {
                    Err(format!("failed to connect to Discord: {err}"))?
                }
            };

            debug!("Connected to Discord, local user is {:#?}", user);

            let discord = Some(discord);
            Ok::<_, Box<dyn Error>>((discord, user, wheel))
        })?;

        let start_time = SystemTime::now();

        let inner = DiscordInner {
            runtime,
            discord,
            _user: user,
            _wheel: wheel,
            start_time,
        };

        let inner = Rc::new(RefCell::new(inner));

        Ok(DiscordPresence(inner))
    }

    pub fn start_activity(
        &mut self,
        activity: ActivityBuilder,
    ) -> Result<Option<Activity>, discord_sdk::Error> {
        self.0.borrow_mut().start_activity(activity)
    }

    pub fn update_activity(
        &self,
        activity: ActivityBuilder,
    ) -> Result<Option<Activity>, discord_sdk::Error> {
        self.0.borrow().update_activity(activity)
    }

    pub fn stop_activity(&self) -> Result<Option<Activity>, discord_sdk::Error> {
        self.0.borrow().stop_activity()
    }
}

impl TryFrom<&str> for DiscordPresence {
    type Error = Box<dyn Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        DiscordPresence::new(value.parse()?)
    }
}

struct DiscordInner {
    runtime: Runtime,
    discord: Option<Discord>,
    _user: User,
    _wheel: Wheel,
    start_time: SystemTime,
}

impl DiscordInner {
    fn start_activity(
        &mut self,
        activity: ActivityBuilder,
    ) -> Result<Option<Activity>, discord_sdk::Error> {
        self.start_time = SystemTime::now();
        self.update_activity(activity)
    }

    fn update_activity(
        &self,
        activity: ActivityBuilder,
    ) -> Result<Option<Activity>, discord_sdk::Error> {
        self.runtime.block_on(async {
            self.discord
                .as_ref()
                .unwrap()
                .update_activity(activity.start_timestamp(self.start_time))
                .await
        })
    }

    fn stop_activity(&self) -> Result<Option<Activity>, discord_sdk::Error> {
        self.runtime
            .block_on(async { self.discord.as_ref().unwrap().clear_activity().await })
    }
}

impl Drop for DiscordInner {
    fn drop(&mut self) {
        self.runtime
            .block_on(async { self.discord.take().unwrap().disconnect().await });
    }
}
