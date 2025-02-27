use activitypub_federation::config::Data;
use actix_web::web::Json;
use lemmy_api_common::{
  community::{BlockCommunity, BlockCommunityResponse},
  context::LemmyContext,
  send_activity::{ActivityChannel, SendActivityData},
};
use lemmy_db_schema::{
  source::{
    community::{CommunityFollower, CommunityFollowerForm},
    community_block::{CommunityBlock, CommunityBlockForm},
  },
  traits::{Blockable, Followable},
};
use lemmy_db_views::structs::{CommunityView, LocalUserView};
use lemmy_utils::error::{LemmyErrorExt, LemmyErrorType, LemmyResult};

pub async fn user_block_community(
  data: Json<BlockCommunity>,
  context: Data<LemmyContext>,
  local_user_view: LocalUserView,
) -> LemmyResult<Json<BlockCommunityResponse>> {
  let community_id = data.community_id;
  let person_id = local_user_view.person.id;
  let community_block_form = CommunityBlockForm {
    person_id,
    community_id,
  };

  if data.block {
    CommunityBlock::block(&mut context.pool(), &community_block_form)
      .await
      .with_lemmy_type(LemmyErrorType::CommunityBlockAlreadyExists)?;

    // Also, unfollow the community, and send a federated unfollow
    let community_follower_form = CommunityFollowerForm::new(data.community_id, person_id);
    CommunityFollower::unfollow(&mut context.pool(), &community_follower_form)
      .await
      .ok();
  } else {
    CommunityBlock::unblock(&mut context.pool(), &community_block_form)
      .await
      .with_lemmy_type(LemmyErrorType::CommunityBlockAlreadyExists)?;
  }

  let community_view = CommunityView::read(
    &mut context.pool(),
    community_id,
    Some(&local_user_view.local_user),
    false,
  )
  .await?;

  ActivityChannel::submit_activity(
    SendActivityData::FollowCommunity(
      community_view.community.clone(),
      local_user_view.person.clone(),
      false,
    ),
    &context,
  )?;

  Ok(Json(BlockCommunityResponse {
    blocked: data.block,
    community_view,
  }))
}
