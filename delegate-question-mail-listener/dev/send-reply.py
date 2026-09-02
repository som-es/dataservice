import os
import smtplib
from email.message import EmailMessage

message = EmailMessage()
message["From"] = os.environ.get("REPLY_FROM", "delegate@somes.local")
message["To"] = os.environ.get("REPLY_TO", "questions@somes.local")
message["Subject"] = os.environ.get("REPLY_SUBJECT", "AW: Ihre Anfrage")

in_reply_to = os.environ.get("IN_REPLY_TO")
if in_reply_to:
    message["In-Reply-To"] = in_reply_to
references = os.environ.get("REFERENCES", in_reply_to)
if references:
    message["References"] = references

message.set_content(os.environ.get("REPLY_BODY", "Vielen Dank, hier meine Antwort."))

host = os.environ.get("SMTP_HOST", "127.0.0.1")
port = int(os.environ.get("SMTP_PORT", "2525"))
with smtplib.SMTP(host, port) as client:
    client.send_message(message)

print(f"sent reply from {message['From']} to {message['To']} in reply to {in_reply_to}")
