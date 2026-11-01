
#include <SoftwareSerial.h>

// ---------------- CONFIGURATION ----------------

#define UART_RX 8
#define UART_TX 9
#define UART_BAUD 9600
#define USB_BAUD 115200

#define MAX_MESSAGE_LENGTH 512
#define MESSAGE_DELIMITER '\n'

// SoftwareSerial(RX, TX)
SoftwareSerial uart(UART_RX, UART_TX);

// ---------------- STATE ----------------

bool ledState = LOW;

char incomingMessage[MAX_MESSAGE_LENGTH];
uint8_t messageIndex = 0;

// ---------------- MACROS ----------------

void text_to_execute(const char* text) {

  if (strcmp(text, "AA") == 0) {
    Serial.println("AA Macro executed");

    // Add AA macro here
  }

  else if (strcmp(text, "BB") == 0) {
    Serial.println("BB Macro executed");

    // Add BB macro here
  }

  else if (strcmp(text, "LED_ON") == 0) {
    digitalWrite(LED_BUILTIN, HIGH);
    Serial.println("LED ON");
  }

  else if (strcmp(text, "LED_OFF") == 0) {
    digitalWrite(LED_BUILTIN, LOW);
    Serial.println("LED OFF");
  }

  else {
    Serial.print("Unknown command: ");
    Serial.println(text);
  }
}

// ---------------- MESSAGE PARSER ----------------

void processMessage() {

  // Terminate string
  incomingMessage[messageIndex] = '\0';

  // Remove Windows carriage return if present
  if (messageIndex > 0 &&
      incomingMessage[messageIndex - 1] == '\r') {
    incomingMessage[messageIndex - 1] = '\0';
  }

  if (messageIndex > 0) {
    Serial.print("Received message: ");
    Serial.println(incomingMessage);

    text_to_execute(incomingMessage);
  }

  // Reset buffer
  messageIndex = 0;
}

// ---------------- UART RECEIVER ----------------

void receiveUART() {

  while (uart.available() > 0) {

    uint8_t receivedByte = uart.read();

    // // HEX debug output
    // Serial.print("0x");
    // if (receivedByte < 0x10) {
    //   Serial.print("0");
    // }
    // Serial.println(receivedByte, HEX);

    // End of message
    if (receivedByte == MESSAGE_DELIMITER) {
      processMessage();
      messageIndex=0;
    }

    else {

      // Prevent buffer overflow
      if (messageIndex < MAX_MESSAGE_LENGTH - 1) {
        incomingMessage[messageIndex++] = (char)receivedByte;
      }

      else {
        Serial.println("ERROR: Message too long");
        messageIndex = 0;
      }
    }
  }
}


// ---------------- SETUP ----------------

void setup() {

  pinMode(LED_BUILTIN, OUTPUT);

  Serial.begin(USB_BAUD);
  uart.begin(UART_BAUD);

  Serial.println();
  Serial.println("UART bridge ready");
}

// ---------------- MAIN LOOP ----------------

void loop() {
  receiveUART();
}
