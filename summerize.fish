#!/opt/homebrew/opt/coreutils/libexec/gnubin/env fish

set json_file "output.json"
set aggregation ""

# Read each post using jq and pass to ollama
jq -c '.[]' $json_file | while read -l post_json
  set post (echo $post_json | jq -r '.post')
  set page (echo $post_json | jq -r '.page')

  # Prepare the AI prompt
  set prompt "# Husqvarna 701 Enduro Information Aggregator Bot\n\nTASK:\n\nYou are an AI bot designed to process and summarize posts from the ADVRider forum specifically about the Husqvarna 701 Enduro motorcycle. Your task is to extract and integrate valuable information from each post into an existing aggregated knowledge base, ensuring there is no duplication of information.\n\nYour focus should be on unique tips, tricks, and insights that would be beneficial for a Husqvarna 701 owner. Exclude common knowledge or basic maintenance tips.\n\nPOST:\n$post\n\nAGGREGATION:\n$aggregation"

  # Call ollama to summarize the post
  set summarized_post (ollama run llama3 "$prompt")

  # Update the aggregation with new summarized information
  set aggregation (echo -e "$aggregation\n$summarized_post")

  echo "Page: $page"
  echo "Post: $post"
  echo "Summarized Post: $summarized_post"
  echo "Aggregation: $aggregation"
  echo "-----------------------------------"
end

# Output the final aggregated knowledge
echo $aggregation

# save to file
echo $aggregation > result.txt
