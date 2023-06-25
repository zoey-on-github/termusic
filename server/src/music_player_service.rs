use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use termusicplayer::player::PlayerExternalCmd;
use termusicplayer::player_service::music_player_server::MusicPlayer;
use termusicplayer::player_service::{
    CycleLoopReply, CycleLoopRequest, EmptyReply, GetProgressRequest, GetProgressResponse,
    PlaySelectedRequest, ReloadConfigRequest, ReloadPlaylistRequest, SeekBackwardRequest,
    SeekForwardRequest, SeekReply, SkipNextRequest, SkipNextResponse, SkipPreviousRequest,
    SpeedDownRequest, SpeedReply, SpeedUpRequest, ToggleGaplessReply, ToggleGaplessRequest,
    TogglePauseRequest, TogglePauseResponse, VolumeDownRequest, VolumeReply, VolumeUpRequest,
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct MusicPlayerService {
    cmd_tx: Arc<Mutex<UnboundedSender<PlayerExternalCmd>>>,
    pub progress: Arc<Mutex<GetProgressResponse>>,
}

impl MusicPlayerService {
    pub fn new(cmd_tx: Arc<Mutex<UnboundedSender<PlayerExternalCmd>>>) -> Self {
        let progress = GetProgressResponse {
            position: 0,
            duration: 60,
            current_track_index: 0,
            status: 1,
            volume: 50,
            speed: 10,
            gapless: true,
        };
        let progress = Arc::new(Mutex::new(progress));

        Self { cmd_tx, progress }
    }
}

impl MusicPlayerService {
    fn command(&self, cmd: &PlayerExternalCmd) {
        if let Err(e) = self.cmd_tx.lock().send(cmd.clone()) {
            error!("error {cmd:?}: {e}");
        }
    }
}

#[tonic::async_trait]
impl MusicPlayer for MusicPlayerService {
    async fn cycle_loop(
        &self,
        _request: Request<CycleLoopRequest>,
    ) -> Result<Response<CycleLoopReply>, Status> {
        let reply = CycleLoopReply {};
        // self.command(&PlayerCommand::CycleLoop);
        Ok(Response::new(reply))
    }
    async fn get_progress(
        &self,
        _request: Request<GetProgressRequest>,
    ) -> Result<Response<GetProgressResponse>, Status> {
        // println!("got a request: {:?}", request);
        let mut reply = GetProgressResponse {
            position: 25,
            duration: 120,
            current_track_index: 0,
            status: 1,
            volume: 50,
            speed: 10,
            gapless: true,
        };
        let r = self.progress.lock();
        reply.position = r.position;
        reply.duration = r.duration;
        reply.current_track_index = r.current_track_index;
        reply.status = r.status;
        reply.volume = r.volume;
        reply.speed = r.speed;
        reply.gapless = r.gapless;

        Ok(Response::new(reply))
    }

    async fn skip_next(
        &self,
        request: Request<SkipNextRequest>,
    ) -> Result<Response<SkipNextResponse>, Status> {
        println!("got a request: {:?}", request);
        let reply = SkipNextResponse {};
        // self.command(&PlayerCmd::SkipNext);
        Ok(Response::new(reply))
    }

    async fn speed_down(
        &self,
        _request: Request<SpeedDownRequest>,
    ) -> Result<Response<SpeedReply>, Status> {
        // self.command(&PlayerCmd::SpeedDown);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = SpeedReply { speed: 10 };
        let s = self.progress.lock();
        reply.speed = s.speed;
        Ok(Response::new(reply))
    }

    async fn speed_up(
        &self,
        _request: Request<SpeedUpRequest>,
    ) -> Result<Response<SpeedReply>, Status> {
        // self.command(&PlayerCmd::SpeedUp);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = SpeedReply { speed: 10 };
        let s = self.progress.lock();
        reply.speed = s.speed;

        Ok(Response::new(reply))
    }

    async fn toggle_pause(
        &self,
        _request: Request<TogglePauseRequest>,
    ) -> Result<Response<TogglePauseResponse>, Status> {
        self.command(&PlayerExternalCmd::TogglePause);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = TogglePauseResponse { status: 1 };
        let r = self.progress.lock();
        reply.status = r.status;
        info!("status returned is: {}", r.status);

        Ok(Response::new(reply))
    }

    async fn volume_down(
        &self,
        _request: Request<VolumeDownRequest>,
    ) -> Result<Response<VolumeReply>, Status> {
        // self.command(&PlayerCmd::VolumeDown);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = VolumeReply { volume: 50 };
        let r = self.progress.lock();
        reply.volume = r.volume;
        info!("volume returned is: {}", r.volume);

        Ok(Response::new(reply))
    }

    async fn volume_up(
        &self,
        _request: Request<VolumeUpRequest>,
    ) -> Result<Response<VolumeReply>, Status> {
        // self.command(&PlayerCmd::VolumeUp);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = VolumeReply { volume: 50 };
        let r = self.progress.lock();
        reply.volume = r.volume;
        info!("volume returned is: {}", r.volume);

        Ok(Response::new(reply))
    }
    async fn toggle_gapless(
        &self,
        request: Request<ToggleGaplessRequest>,
    ) -> Result<Response<ToggleGaplessReply>, Status> {
        println!("got a request: {:?}", request);
        // self.command(&PlayerCmd::ToggleGapless);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = ToggleGaplessReply { gapless: true };
        let r = self.progress.lock();
        reply.gapless = r.gapless;
        info!("gapless returned is: {}", r.gapless);

        Ok(Response::new(reply))
    }

    async fn seek_forward(
        &self,
        _request: Request<SeekForwardRequest>,
    ) -> Result<Response<SeekReply>, Status> {
        // self.command(&PlayerCmd::SeekForward);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = SeekReply {
            position: 0,
            duration: 60,
        };
        let s = self.progress.lock();
        reply.position = s.position;
        reply.duration = s.duration;

        Ok(Response::new(reply))
    }

    async fn seek_backward(
        &self,
        _request: Request<SeekBackwardRequest>,
    ) -> Result<Response<SeekReply>, Status> {
        // self.command(&PlayerCmd::SeekBackward);
        // This is to let the player update volume within loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut reply = SeekReply {
            position: 0,
            duration: 60,
        };
        let s = self.progress.lock();
        reply.position = s.position;
        reply.duration = s.duration;

        Ok(Response::new(reply))
    }

    async fn reload_config(
        &self,
        _request: Request<ReloadConfigRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let reply = EmptyReply {};
        // self.command(&PlayerCmd::ReloadConfig);
        Ok(Response::new(reply))
    }

    async fn reload_playlist(
        &self,
        _request: Request<ReloadPlaylistRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let reply = EmptyReply {};
        // self.command(&PlayerCmd::ReloadPlaylist);
        Ok(Response::new(reply))
    }

    async fn play_selected(
        &self,
        _request: Request<PlaySelectedRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let reply = EmptyReply {};
        // self.command(&PlayerCmd::PlaySelected);
        Ok(Response::new(reply))
    }

    async fn skip_previous(
        &self,
        _request: Request<SkipPreviousRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let reply = EmptyReply {};
        // self.command(&PlayerCmd::SkipPrevious);
        Ok(Response::new(reply))
    }
}
