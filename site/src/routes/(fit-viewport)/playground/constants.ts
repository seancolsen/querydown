export const starting_querydown = `#issues
created_at:>@6M|ago
--#assignments
++#labels{name:..["Regression" "Bug"]}
10..20:#comments{user.team.name!"Backend"}
$*
$author.username
$#comments.created_at%min \\sd
`;